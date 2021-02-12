use crate::vault_types::{FfiSecretAttributes, SecretKeyHandle};
use crate::{check_buffer, FfiError, FfiObjectMutexStorage, FfiOckamError};
use crate::{FfiVaultFatPointer, FfiVaultType};
use lazy_static::lazy_static;
use ockam_core::lib::convert::{TryFrom, TryInto};
use ockam_core::lib::slice;
use ockam_vault::SoftwareVault;
use ockam_vault_core::{
    AsymmetricVault, HashVault, PublicKey, Secret, SecretAttributes, SecretType, SecretVault,
    SymmetricVault,
};
use std::ops::DerefMut;
use std::sync::{Arc, Mutex};

pub trait FfiVault: SecretVault + HashVault + SymmetricVault + AsymmetricVault + Send {}

impl<D> FfiVault for D where D: SecretVault + HashVault + SymmetricVault + AsymmetricVault + Send {}

lazy_static! {
    pub(crate) static ref DEFAULT_VAULTS: FfiObjectMutexStorage<SoftwareVault> =
        FfiObjectMutexStorage::default();
}

fn call<F, R>(context: FfiVaultFatPointer, callback: F) -> Result<R, FfiOckamError>
where
    F: FnOnce(&mut dyn FfiVault) -> Result<R, FfiOckamError>,
{
    match context.vault_type() {
        FfiVaultType::Software => {
            let item = DEFAULT_VAULTS.get_object(context.handle())?;
            let mut item = item.lock().unwrap();

            callback(item.deref_mut())
        }
    }
}

/// Create a new Ockam Default vault and return it
#[no_mangle]
pub extern "C" fn ockam_vault_default_init(context: &mut FfiVaultFatPointer) -> FfiOckamError {
    // TODO: handle logging
    let handle = match DEFAULT_VAULTS.insert_object(Arc::new(Mutex::new(SoftwareVault::default())))
    {
        Ok(handle) => handle,
        Err(err) => return err,
    };

    *context = FfiVaultFatPointer::new(handle, FfiVaultType::Software);

    FfiOckamError::none()
}

/// Compute the SHA-256 hash on `input` and put the result in `digest`.
/// `digest` must be 32 bytes in length
#[no_mangle]
pub extern "C" fn ockam_vault_sha256(
    context: FfiVaultFatPointer,
    input: *const u8,
    input_length: u32,
    digest: *mut u8,
) -> FfiOckamError {
    check_buffer!(input);
    check_buffer!(digest);

    let input = unsafe { std::slice::from_raw_parts(input, input_length as usize) };

    match call(context, |v| -> Result<(), FfiOckamError> {
        let d = v.sha256(input)?;

        unsafe {
            std::ptr::copy_nonoverlapping(d.as_ptr(), digest, d.len());
        }

        Ok(())
    }) {
        Ok(_) => FfiOckamError::none(),
        Err(err) => err.into(),
    }
}

/// Generate a secret key with the specific attributes.
/// Returns a handle for the secret
#[no_mangle]
pub extern "C" fn ockam_vault_secret_generate(
    context: FfiVaultFatPointer,
    secret: &mut SecretKeyHandle,
    attributes: FfiSecretAttributes,
) -> FfiOckamError {
    *secret = match call(context, |v| -> Result<SecretKeyHandle, FfiOckamError> {
        let atts = attributes.try_into()?;
        let ctx = v.secret_generate(atts)?;
        Ok(ctx.index() as u64)
    }) {
        Ok(h) => h,
        Err(err) => return err.into(),
    };

    FfiOckamError::none()
}

/// Import a secret key with the specific handle and attributes
#[no_mangle]
pub extern "C" fn ockam_vault_secret_import(
    context: FfiVaultFatPointer,
    secret: &mut SecretKeyHandle,
    attributes: FfiSecretAttributes,
    input: *mut u8,
    input_length: u32,
) -> FfiOckamError {
    check_buffer!(input, input_length);

    *secret = match call(context, |v| -> Result<SecretKeyHandle, FfiOckamError> {
        let atts = attributes.try_into()?;
        let secret_data = unsafe { std::slice::from_raw_parts(input, input_length as usize) };

        let ctx = v.secret_import(secret_data, atts)?;
        Ok(ctx.index() as u64)
    }) {
        Ok(s) => s,
        Err(err) => return err.into(),
    };

    FfiOckamError::none()
}

/// Export a secret key with the specific handle to the output buffer
#[no_mangle]
pub extern "C" fn ockam_vault_secret_export(
    context: FfiVaultFatPointer,
    secret: SecretKeyHandle,
    output_buffer: &mut u8,
    output_buffer_size: u32,
    output_buffer_length: &mut u32,
) -> FfiOckamError {
    *output_buffer_length = 0;
    match call(context, |v| -> Result<(), FfiOckamError> {
        let ctx = Secret::new(secret as usize);
        let key = v.secret_export(&ctx)?;
        if output_buffer_size < key.as_ref().len() as u32 {
            return Err(FfiError::BufferTooSmall.into());
        }
        *output_buffer_length = key.as_ref().len() as u32;
        unsafe {
            std::ptr::copy_nonoverlapping(key.as_ref().as_ptr(), output_buffer, key.as_ref().len());
        };
        Ok(())
    }) {
        Ok(_) => FfiOckamError::none(),
        Err(err) => err.into(),
    }
}

/// Get the public key from a secret key to the output buffer
#[no_mangle]
pub extern "C" fn ockam_vault_secret_publickey_get(
    context: FfiVaultFatPointer,
    secret: SecretKeyHandle,
    output_buffer: &mut u8,
    output_buffer_size: u32,
    output_buffer_length: &mut u32,
) -> FfiOckamError {
    *output_buffer_length = 0;
    match call(context, |v| -> Result<(), FfiOckamError> {
        let ctx = Secret::new(secret as usize);
        let key = v.secret_public_key_get(&ctx)?;
        if output_buffer_size < key.as_ref().len() as u32 {
            return Err(FfiError::BufferTooSmall.into());
        }
        *output_buffer_length = key.as_ref().len() as u32;
        unsafe {
            std::ptr::copy_nonoverlapping(key.as_ref().as_ptr(), output_buffer, key.as_ref().len());
        };
        Ok(())
    }) {
        Ok(_) => FfiOckamError::none(),
        Err(err) => err.into(),
    }
}

/// Retrieve the attributes for a specified secret
#[no_mangle]
pub extern "C" fn ockam_vault_secret_attributes_get(
    context: FfiVaultFatPointer,
    secret: SecretKeyHandle,
    attributes: &mut FfiSecretAttributes,
) -> FfiOckamError {
    *attributes = match call(context, |v| -> Result<FfiSecretAttributes, FfiOckamError> {
        let ctx = Secret::new(secret as usize);
        let atts = v.secret_attributes_get(&ctx)?;
        Ok(atts.into())
    }) {
        Ok(a) => a,
        Err(err) => return err.into(),
    };

    FfiOckamError::none()
}

/// Delete an ockam vault secret.
#[no_mangle]
pub extern "C" fn ockam_vault_secret_destroy(
    context: FfiVaultFatPointer,
    secret: SecretKeyHandle,
) -> FfiOckamError {
    match call(context, |v| -> Result<(), FfiOckamError> {
        let ctx = Secret::new(secret as usize);
        v.secret_destroy(ctx)?;
        Ok(())
    }) {
        Ok(_) => FfiOckamError::none(),
        Err(err) => err.into(),
    }
}

/// Perform an ECDH operation on the supplied ockam vault secret and peer_publickey. The result is
/// another ockam vault secret of type unknown.
#[no_mangle]
pub extern "C" fn ockam_vault_ecdh(
    context: FfiVaultFatPointer,
    secret: SecretKeyHandle,
    peer_publickey: *const u8,
    peer_publickey_length: u32,
    shared_secret: &mut SecretKeyHandle,
) -> FfiOckamError {
    check_buffer!(peer_publickey, peer_publickey_length);
    let peer_publickey =
        unsafe { std::slice::from_raw_parts(peer_publickey, peer_publickey_length as usize) };
    *shared_secret = match call(context, |v| -> Result<u64, FfiOckamError> {
        let ctx = Secret::new(secret as usize);
        let atts = v.secret_attributes_get(&ctx)?;
        let pubkey = match atts.stype() {
            SecretType::Curve25519 => {
                if peer_publickey.len() != 32 {
                    Err(FfiError::InvalidPublicKey)
                } else {
                    Ok(PublicKey::new(peer_publickey.to_vec()))
                }
            }
            SecretType::P256 => {
                if peer_publickey.len() != 65 {
                    Err(FfiError::InvalidPublicKey)
                } else {
                    Ok(PublicKey::new(peer_publickey.to_vec()))
                }
            }
            _ => Err(FfiError::UnknownPublicKeyType),
        }?;
        let shared_ctx = v.ec_diffie_hellman(&ctx, pubkey.as_ref())?;
        Ok(shared_ctx.index() as u64)
    }) {
        Ok(s) => s,
        Err(err) => return err.into(),
    };

    FfiOckamError::none()
}

/// Perform an HMAC-SHA256 based key derivation function on the supplied salt and input key
/// material.
#[no_mangle]
pub extern "C" fn ockam_vault_hkdf_sha256(
    context: FfiVaultFatPointer,
    salt: SecretKeyHandle,
    input_key_material: *const SecretKeyHandle,
    derived_outputs_attributes: *const FfiSecretAttributes,
    derived_outputs_count: u8,
    derived_outputs: *mut SecretKeyHandle,
) -> FfiOckamError {
    let derived_outputs_count = derived_outputs_count as usize;
    match call(context, |v| -> Result<(), FfiOckamError> {
        let salt_ctx = Secret::new(salt as usize);
        let ikm_ctx = if input_key_material.is_null() {
            None
        } else {
            let ctx = unsafe { Secret::new(*input_key_material as usize) };
            Some(ctx)
        };
        let ikm_ctx = ikm_ctx.as_ref();
        let array: &[FfiSecretAttributes] =
            unsafe { slice::from_raw_parts(derived_outputs_attributes, derived_outputs_count) };

        let mut output_attributes = Vec::<SecretAttributes>::with_capacity(array.len());
        for x in array.iter() {
            output_attributes.push(SecretAttributes::try_from(*x)?);
        }

        // TODO: Hardcoded to be empty for now because any changes
        // to the C layer requires an API change.
        // This change was necessary to implement Enrollment since the info string is not
        // left blank for that protocol, but is blank for the XX key exchange pattern.
        // If we agree to change the API, then this wouldn't be hardcoded but received
        // from a parameter in the C API. Elixir and other consumers would be expected
        // to pass the appropriate flag. The other option is to not expose the vault
        // directly since it may confuse users about what to pass here and
        // I don't like the idea of yelling at consumers through comments.
        // Instead the vault could be encapsulated in channels and key exchanges.
        // Either way, I don't want to change the API until this decision is finalized.
        let hkdf_output = v.hkdf_sha256(&salt_ctx, b"", ikm_ctx, output_attributes)?;

        let hkdf_output: Vec<SecretKeyHandle> =
            hkdf_output.into_iter().map(|x| x.index() as u64).collect();

        unsafe {
            std::ptr::copy_nonoverlapping(
                hkdf_output.as_ptr(),
                derived_outputs,
                derived_outputs_count,
            )
        };
        Ok(())
    }) {
        Ok(_) => FfiOckamError::none(),
        Err(err) => err.into(),
    }
}

///   Encrypt a payload using AES-GCM.
#[no_mangle]
pub extern "C" fn ockam_vault_aead_aes_gcm_encrypt(
    context: FfiVaultFatPointer,
    secret: SecretKeyHandle,
    nonce: u16,
    additional_data: *const u8,
    additional_data_length: u32,
    plaintext: *const u8,
    plaintext_length: u32,
    ciphertext_and_tag: &mut u8,
    ciphertext_and_tag_size: u32,
    ciphertext_and_tag_length: &mut u32,
) -> FfiOckamError {
    check_buffer!(additional_data);
    check_buffer!(plaintext);
    *ciphertext_and_tag_length = 0;
    let additional_data =
        unsafe { std::slice::from_raw_parts(additional_data, additional_data_length as usize) };
    let plaintext = unsafe { std::slice::from_raw_parts(plaintext, plaintext_length as usize) };
    match call(context, |v| -> Result<(), FfiOckamError> {
        let ctx = Secret::new(secret as usize);
        let mut nonce_vec = vec![0; 12 - 2];
        nonce_vec.extend_from_slice(&nonce.to_be_bytes());
        let ciphertext = v.aead_aes_gcm_encrypt(&ctx, plaintext, &nonce_vec, additional_data)?;

        if ciphertext_and_tag_size < ciphertext.len() as u32 {
            return Err(FfiError::BufferTooSmall.into());
        }
        *ciphertext_and_tag_length = ciphertext.len() as u32;
        unsafe {
            std::ptr::copy_nonoverlapping(ciphertext.as_ptr(), ciphertext_and_tag, ciphertext.len())
        };
        Ok(())
    }) {
        Ok(_) => FfiOckamError::none(),
        Err(err) => err.into(),
    }
}

/// Decrypt a payload using AES-GCM.
#[no_mangle]
pub extern "C" fn ockam_vault_aead_aes_gcm_decrypt(
    context: FfiVaultFatPointer,
    secret: SecretKeyHandle,
    nonce: u16,
    additional_data: *const u8,
    additional_data_length: u32,
    ciphertext_and_tag: *const u8,
    ciphertext_and_tag_length: u32,
    plaintext: &mut u8,
    plaintext_size: u32,
    plaintext_length: &mut u32,
) -> FfiOckamError {
    check_buffer!(ciphertext_and_tag, ciphertext_and_tag_length);
    check_buffer!(additional_data);
    *plaintext_length = 0;
    let additional_data =
        unsafe { std::slice::from_raw_parts(additional_data, additional_data_length as usize) };
    let ciphertext_and_tag = unsafe {
        std::slice::from_raw_parts(ciphertext_and_tag, ciphertext_and_tag_length as usize)
    };
    match call(context, |v| -> Result<(), FfiOckamError> {
        let ctx = Secret::new(secret as usize);
        let mut nonce_vec = vec![0; 12 - 2];
        nonce_vec.extend_from_slice(&nonce.to_be_bytes());
        let plain =
            v.aead_aes_gcm_decrypt(&ctx, ciphertext_and_tag, &nonce_vec, additional_data)?;
        if plaintext_size < plain.len() as u32 {
            return Err(FfiError::BufferTooSmall.into());
        }
        *plaintext_length = plain.len() as u32;
        unsafe { std::ptr::copy_nonoverlapping(plain.as_ptr(), plaintext, plain.len()) };
        Ok(())
    }) {
        Ok(_) => FfiOckamError::none(),
        Err(err) => err.into(),
    }
}

/// Deinitialize an Ockam vault
#[no_mangle]
pub extern "C" fn ockam_vault_deinit(context: FfiVaultFatPointer) -> FfiOckamError {
    match context.vault_type() {
        FfiVaultType::Software => match DEFAULT_VAULTS.remove_object(context.handle()) {
            Ok(_) => FfiOckamError::none(),
            Err(_) => FfiError::VaultNotFound.into(),
        },
    }
}
