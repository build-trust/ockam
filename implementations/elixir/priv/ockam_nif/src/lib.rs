use std::sync::{Mutex, RwLock};

use rustler;
use rustler::types::{Binary, OwnedBinary};
use rustler::{Env, ResourceArc};

use ockam::vault::*;

/// Wraps a Vault in a lock to protect it when shared across threads
pub struct VaultResource {
    vault: Mutex<Vault>,
}
unsafe impl Send for VaultResource {}
unsafe impl Sync for VaultResource {}
impl VaultResource {
    pub fn new() -> VaultResult<Self> {
        let vault = Vault::new()?;
        Ok(Self {
            vault: Mutex::new(vault),
        })
    }
}

/// Wraps a Secret in a r/w lock to ensure that set_type/destroy are performed only
/// when exclusive access is held; all other usages are read-only.
///
/// In addition, this resource holds a reference to the VaultResource it was pulled
/// from, to prevent the Vault from being destroyed with secrets still being used.
pub struct SecretResource {
    vault: ResourceArc<VaultResource>,
    secret: RwLock<Secret>,
}
unsafe impl Send for SecretResource {}
unsafe impl Sync for SecretResource {}
impl SecretResource {
    pub fn new(vault: ResourceArc<VaultResource>, secret: Secret) -> Self {
        Self {
            vault,
            secret: RwLock::new(secret),
        }
    }
}
impl Drop for SecretResource {
    fn drop(&mut self) {
        let mut secret = self.secret.get_mut().unwrap();
        let mut vault = self.vault.vault.lock().unwrap();
        let _ = vault.destroy_secret(&mut secret).ok();
        drop(vault);
    }
}

/// Represents the result of operations which return `:ok | :error`
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MayFail {
    Ok,
    Error,
}
impl From<VaultResult<()>> for MayFail {
    fn from(res: VaultResult<()>) -> Self {
        match res {
            Ok(_) => Self::Ok,
            Err(_) => Self::Error,
        }
    }
}
impl rustler::Encoder for MayFail {
    fn encode<'c>(&self, env: rustler::Env<'c>) -> rustler::Term<'c> {
        use rustler::Atom;
        match self {
            Self::Ok => Atom::from_str(env, "ok").unwrap().to_term(env),
            Self::Error => Atom::from_str(env, "error").unwrap().to_term(env),
        }
    }
}

#[rustler::nif]
pub fn make_vault() -> VaultResult<ResourceArc<VaultResource>> {
    Ok(ResourceArc::new(VaultResource::new()?))
}

#[rustler::nif]
pub fn random(resource: ResourceArc<VaultResource>) -> i32 {
    use rand::prelude::*;

    let mut vault = resource.vault.lock().unwrap();
    vault.gen()
}

#[rustler::nif]
pub fn sha256<'a>(
    env: Env<'a>,
    resource: ResourceArc<VaultResource>,
    data: Binary,
) -> VaultResult<Binary<'a>> {
    let mut buffer = OwnedBinary::new(32).unwrap();
    let mut vault = resource.vault.lock().unwrap();
    let bytes_written = vault.sha256_with_buffer(data.as_slice(), buffer.as_mut_slice())?;

    if bytes_written < 32 {
        let _ = buffer.realloc(bytes_written);
    }

    Ok(buffer.release(env))
}

#[rustler::nif]
pub fn generate_secret(
    vault_resource: ResourceArc<VaultResource>,
    attributes: SecretAttributes,
) -> VaultResult<ResourceArc<SecretResource>> {
    let mut vault = vault_resource.vault.lock().unwrap();
    vault
        .generate_secret(attributes)
        .map(|s| ResourceArc::new(SecretResource::new(vault_resource.clone(), s)))
}

#[rustler::nif]
pub fn import_secret(
    vault_resource: ResourceArc<VaultResource>,
    input: Binary,
    attributes: SecretAttributes,
) -> VaultResult<ResourceArc<SecretResource>> {
    let mut vault = vault_resource.vault.lock().unwrap();
    vault
        .import_secret(input.as_slice(), attributes)
        .map(|s| ResourceArc::new(SecretResource::new(vault_resource.clone(), s)))
}

#[rustler::nif]
pub fn export_secret<'a>(
    env: Env<'a>,
    vault_resource: ResourceArc<VaultResource>,
    secret_resource: ResourceArc<SecretResource>,
) -> VaultResult<Binary<'a>> {
    let secret = secret_resource.secret.read().unwrap();
    let mut vault = vault_resource.vault.lock().unwrap();
    let attrs = vault.get_secret_attributes(&secret)?;
    let mut buffer = OwnedBinary::new(attrs.length as usize).unwrap();
    let bytes_written = vault.export_secret_with_buffer(&secret, buffer.as_mut_slice())?;

    if bytes_written < attrs.length as usize {
        let _ = buffer.realloc(bytes_written);
    }

    Ok(buffer.release(env))
}

#[rustler::nif]
pub fn get_secret_attributes(
    vault_resource: ResourceArc<VaultResource>,
    secret_resource: ResourceArc<SecretResource>,
) -> VaultResult<SecretAttributes> {
    let secret = secret_resource.secret.read().unwrap();
    let mut vault = vault_resource.vault.lock().unwrap();
    vault.get_secret_attributes(&secret)
}

#[rustler::nif]
pub fn get_public_key<'a>(
    env: Env<'a>,
    vault_resource: ResourceArc<VaultResource>,
    secret_resource: ResourceArc<SecretResource>,
) -> VaultResult<Binary<'a>> {
    let secret = secret_resource.secret.read().unwrap();
    let mut vault = vault_resource.vault.lock().unwrap();
    let attrs = vault.get_secret_attributes(&secret)?;
    let mut buffer = OwnedBinary::new(attrs.length as usize).unwrap();
    let bytes_written = vault.get_public_key_with_buffer(&secret, buffer.as_mut_slice())?;

    if bytes_written < attrs.length as usize {
        let _ = buffer.realloc(bytes_written);
    }

    Ok(buffer.release(env))
}

#[rustler::nif]
pub fn set_secret_type(
    vault_resource: ResourceArc<VaultResource>,
    secret_resource: ResourceArc<SecretResource>,
    ty: SecretType,
) -> MayFail {
    let mut secret = secret_resource.secret.write().unwrap();
    let mut vault = vault_resource.vault.lock().unwrap();
    vault.set_secret_type(&mut secret, ty).into()
}

#[rustler::nif]
pub fn ecdh(
    vault_resource: ResourceArc<VaultResource>,
    privkey_resource: ResourceArc<SecretResource>,
    peer_pubkey: Binary,
) -> VaultResult<ResourceArc<SecretResource>> {
    let privkey = privkey_resource.secret.read().unwrap();
    let mut vault = vault_resource.vault.lock().unwrap();
    vault
        .ecdh(&privkey, peer_pubkey.as_slice())
        .map(|s| ResourceArc::new(SecretResource::new(vault_resource.clone(), s)))
}

#[rustler::nif]
pub fn hkdf_sha256(
    vault_resource: ResourceArc<VaultResource>,
    salt_resource: ResourceArc<SecretResource>,
    input_key_material_resource: Option<ResourceArc<SecretResource>>,
    num_derived_outputs: u8,
) -> VaultResult<Vec<ResourceArc<SecretResource>>> {
    let salt = salt_resource.secret.read().unwrap();
    let mut secrets = match input_key_material_resource {
        None => {
            let mut vault = vault_resource.vault.lock().unwrap();
            vault.hkdf_sha256(&salt, None, num_derived_outputs)?
        }
        Some(ikmr) => {
            let input_key_material = ikmr.secret.read().unwrap();
            let mut vault = vault_resource.vault.lock().unwrap();
            vault.hkdf_sha256(&salt, Some(&input_key_material), num_derived_outputs)?
        }
    };

    let secret_resources = secrets
        .drain(..)
        .map(|s| ResourceArc::new(SecretResource::new(vault_resource.clone(), s)))
        .collect::<Vec<_>>();

    Ok(secret_resources)
}

#[rustler::nif]
pub fn aead_aes_gcm_encrypt<'a>(
    env: Env<'a>,
    vault_resource: ResourceArc<VaultResource>,
    key_resource: ResourceArc<SecretResource>,
    nonce: u16,
    additional_data: Option<Binary>,
    plaintext: Binary,
) -> VaultResult<Binary<'a>> {
    let key = key_resource.secret.read().unwrap();
    let mut vault = vault_resource.vault.lock().unwrap();
    let plaintext_slice = plaintext.as_slice();
    let len = plaintext_slice.len() + 16;
    let mut buffer = OwnedBinary::new(len).unwrap();
    let bytes_written = vault.aead_aes_gcm_encrypt_with_buffer(
        &key,
        nonce,
        additional_data.map(|d| d.as_slice()),
        plaintext_slice,
        buffer.as_mut_slice(),
    )?;
    if bytes_written < len {
        let _ = buffer.realloc(bytes_written);
    }
    Ok(buffer.release(env))
}

#[rustler::nif]
pub fn aead_aes_gcm_decrypt<'a>(
    env: Env<'a>,
    vault_resource: ResourceArc<VaultResource>,
    key_resource: ResourceArc<SecretResource>,
    nonce: u16,
    additional_data: Option<Binary>,
    ciphertext_and_tag: Binary,
) -> VaultResult<Binary<'a>> {
    let key = key_resource.secret.read().unwrap();
    let mut vault = vault_resource.vault.lock().unwrap();
    let ciphertext_and_tag_slice = ciphertext_and_tag.as_slice();
    let len = ciphertext_and_tag_slice.len() - 16;
    let mut buffer = OwnedBinary::new(len).unwrap();
    let bytes_written = vault.aead_aes_gcm_decrypt_with_buffer(
        &key,
        nonce,
        additional_data.map(|d| d.as_slice()),
        ciphertext_and_tag_slice,
        buffer.as_mut_slice(),
    )?;
    if bytes_written < len {
        let _ = buffer.realloc(bytes_written);
    }
    Ok(buffer.release(env))
}

rustler::init!(
    "Elixir.Ockam.Vault.NIF",
    [
        make_vault,
        random,
        sha256,
        generate_secret,
        import_secret,
        export_secret,
        get_secret_attributes,
        set_secret_type,
        get_public_key,
        ecdh,
        hkdf_sha256,
        aead_aes_gcm_encrypt,
        aead_aes_gcm_decrypt,
    ],
    load = on_load
);

fn on_load(env: Env, _: rustler::Term) -> bool {
    rustler::resource!(VaultResource, env);
    rustler::resource!(SecretResource, env);
    true
}
