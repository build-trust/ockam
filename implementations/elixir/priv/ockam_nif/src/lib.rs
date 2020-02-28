use std::sync::RwLock;

use rustler;
use rustler::types::{Binary, OwnedBinary};
use rustler::{Env, ResourceArc};

use ockam::vault::{Curve, KeyType, Vault, VaultFeatures, VaultResult};

pub struct VaultResource {
    vault: RwLock<Vault>,
}
unsafe impl Send for VaultResource {}
unsafe impl Sync for VaultResource {}
impl VaultResource {
    pub fn new(curve: Curve) -> Result<Self, ()> {
        let features = VaultFeatures::OCKAM_VAULT_FEATURE_ALL;
        let vault = Vault::new(features, curve)?;
        Ok(Self {
            vault: RwLock::new(vault),
        })
    }
}

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
pub fn make_vault(curve: Curve) -> Result<ResourceArc<VaultResource>, ()> {
    Ok(ResourceArc::new(VaultResource::new(curve)?))
}

#[rustler::nif]
pub fn random(resource: ResourceArc<VaultResource>) -> i32 {
    use rand::prelude::*;

    let mut vault = resource.vault.write().unwrap();
    vault.gen()
}

#[rustler::nif]
pub fn key_gen(resource: ResourceArc<VaultResource>, key_type: KeyType) -> MayFail {
    let mut vault = resource.vault.write().unwrap();
    vault.key_gen(key_type).into()
}

#[rustler::nif]
pub fn get_public_key<'a>(
    env: Env<'a>,
    resource: ResourceArc<VaultResource>,
    key_type: KeyType,
) -> VaultResult<Binary<'a>> {
    let mut buffer = OwnedBinary::new(32).unwrap();
    let mut vault = resource.vault.write().unwrap();
    vault.get_public_key_with_buffer(key_type, buffer.as_mut_slice())?;
    Ok(buffer.release(env))
}

#[rustler::nif]
pub fn write_public_key(
    resource: ResourceArc<VaultResource>,
    key_type: KeyType,
    privkey: Binary,
) -> MayFail {
    let mut vault = resource.vault.write().unwrap();
    vault.write_private_key(key_type, privkey.as_slice()).into()
}

#[rustler::nif]
pub fn ecdh<'a>(
    env: Env<'a>,
    resource: ResourceArc<VaultResource>,
    key_type: KeyType,
    pubkey: Binary,
) -> VaultResult<Binary<'a>> {
    let mut buffer = OwnedBinary::new(32).unwrap();
    let mut vault = resource.vault.write().unwrap();
    vault.ecdh_with_buffer(key_type, pubkey.as_slice(), buffer.as_mut_slice())?;
    Ok(buffer.release(env))
}

#[rustler::nif]
pub fn hkdf<'a>(
    env: Env<'a>,
    resource: ResourceArc<VaultResource>,
    salt: Binary,
    key: Binary,
    info: Option<Binary>,
) -> VaultResult<Binary<'a>> {
    let salt_slice = salt.as_slice();
    let key_slice = key.as_slice();
    let info = info.map(|i| i.as_slice());
    let mut buffer = OwnedBinary::new(32).unwrap();
    let mut vault = resource.vault.write().unwrap();
    vault.hkdf_with_buffer(salt_slice, key_slice, info, buffer.as_mut_slice())?;
    Ok(buffer.release(env))
}

#[rustler::nif]
pub fn aes_gcm_encrypt<'a>(
    env: Env<'a>,
    resource: ResourceArc<VaultResource>,
    input: Binary,
    key: Binary,
    iv: Binary,
    additional_data: Option<Binary>,
    tag: Binary,
) -> VaultResult<Binary<'a>> {
    let input_slice = input.as_slice();
    let key_slice = key.as_slice();
    let iv_slice = iv.as_slice();
    let additional_data = additional_data.map(|d| d.as_slice());
    let tag_slice = tag.as_slice();
    let mut buffer = OwnedBinary::new(input_slice.len()).unwrap();
    let mut vault = resource.vault.write().unwrap();
    vault.aes_gcm_encrypt_with_buffer(
        input_slice,
        key_slice,
        iv_slice,
        additional_data,
        tag_slice,
        buffer.as_mut_slice(),
    )?;
    Ok(buffer.release(env))
}

#[rustler::nif]
pub fn aes_gcm_decrypt<'a>(
    env: Env<'a>,
    resource: ResourceArc<VaultResource>,
    input: Binary,
    key: Binary,
    iv: Binary,
    additional_data: Option<Binary>,
    tag: Binary,
) -> VaultResult<Binary<'a>> {
    let input_slice = input.as_slice();
    let key_slice = key.as_slice();
    let iv_slice = iv.as_slice();
    let additional_data = additional_data.map(|d| d.as_slice());
    let tag_slice = tag.as_slice();
    let mut buffer = OwnedBinary::new(input_slice.len()).unwrap();
    let mut vault = resource.vault.write().unwrap();
    vault.aes_gcm_decrypt_with_buffer(
        input_slice,
        key_slice,
        iv_slice,
        additional_data,
        tag_slice,
        buffer.as_mut_slice(),
    )?;
    Ok(buffer.release(env))
}

rustler::init!(
    "Elixir.Ockam.Vault.NIF",
    [
        make_vault,
        random,
        key_gen,
        get_public_key,
        write_public_key,
        ecdh,
        hkdf,
        aes_gcm_encrypt,
        aes_gcm_decrypt,
    ],
    load = on_load
);

fn on_load(env: Env, _: rustler::Term) -> bool {
    rustler::resource!(VaultResource, env);
    true
}
