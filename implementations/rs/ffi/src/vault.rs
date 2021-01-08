use crate::default_vault_adapter::DefaultVaultAdapter;
use crate::error::{Error, FfiOckamError};
use crate::mutex_storage::FfiObjectMutexStorage;
use crate::nomutex_storage::FfiObjectNoMutexStorage;
use crate::vault_types::*;
use ockam_vault_file::FilesystemVault;
use ockam_vault_software::ockam_vault::types::{PublicKey, SecretAttributes, SecretType};
use ockam_vault_software::ockam_vault::{
    AsymmetricVault, HashVault, PersistentVault, Secret, SecretVault, SymmetricVault,
};
use ockam_vault_software::DefaultVault;
use std::ffi::{CStr, CString};
use std::ops::DerefMut;
use std::os::raw::c_char;
use std::slice;
use std::sync::{Arc, Mutex};

pub trait FfiVault:
    SecretVault + HashVault + SymmetricVault + AsymmetricVault + PersistentVault + Send
{
}

impl<D> FfiVault for D where
    D: SecretVault + HashVault + SymmetricVault + AsymmetricVault + PersistentVault + Send
{
}

lazy_static! {
    pub(crate) static ref DEFAULT_VAULTS: FfiObjectMutexStorage<DefaultVaultAdapter> =
        FfiObjectMutexStorage::default();
    pub(crate) static ref FILESYSTEM_VAULTS: FfiObjectMutexStorage<FilesystemVault> =
        FfiObjectMutexStorage::default();
    pub(crate) static ref SECRETS: FfiObjectNoMutexStorage<Box<dyn Secret>> =
        FfiObjectNoMutexStorage::default();
}

fn call<F, R>(context: FfiVaultFatPointer, callback: F) -> Result<R, FfiOckamError>
where
    F: FnOnce(&mut dyn FfiVault) -> Result<R, FfiOckamError>,
{
    match context.vault_type {
        FfiVaultType::Software => {
            let item = DEFAULT_VAULTS.get_object(context.handle)?;
            let mut item = item.lock().unwrap();

            callback(item.deref_mut())
        }
        FfiVaultType::Filesystem => {
            let item = FILESYSTEM_VAULTS.get_object(context.handle)?;
            let mut item = item.lock().unwrap();

            callback(item.deref_mut())
        }
    }
}

/// Create a new Ockam Default vault and return it
#[no_mangle]
pub extern "C" fn ockam_vault_default_init(context: &mut FfiVaultFatPointer) -> FfiOckamError {
    // TODO: handle logging
    let handle = match DEFAULT_VAULTS.insert_object(Arc::new(Mutex::new(DefaultVaultAdapter::new(
        DefaultVault::default(),
    )))) {
        Ok(handle) => handle,
        Err(err) => return err,
    };

    *context = FfiVaultFatPointer {
        handle,
        vault_type: FfiVaultType::Software,
    };

    FfiOckamError::none()
}

/// Create a new Ockam file vault and return it
#[no_mangle]
pub extern "C" fn ockam_vault_file_init(
    context: &mut FfiVaultFatPointer,
    path: *const c_char,
) -> FfiOckamError {
    let path = unsafe { CStr::from_ptr(path) }
        .to_string_lossy()
        .into_owned();
    let vault = match FilesystemVault::new(std::path::PathBuf::from(path)) {
        Ok(v) => v,
        Err(_) => return Error::ErrorCreatingFilesystemVault.into(),
    };
    let handle = match FILESYSTEM_VAULTS.insert_object(Arc::new(Mutex::new(vault))) {
        Ok(handle) => handle,
        Err(err) => return err,
    };

    *context = FfiVaultFatPointer {
        handle,
        vault_type: FfiVaultType::Filesystem,
    };

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
    let atts = attributes.into();
    *secret = match call(context, |v| -> Result<SecretKeyHandle, FfiOckamError> {
        let ctx = v.secret_generate(atts)?;
        Ok(SECRETS.insert_object(ctx)?)
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

    let atts = attributes.into();
    *secret = match call(context, |v| -> Result<SecretKeyHandle, FfiOckamError> {
        let secret_data = unsafe { std::slice::from_raw_parts(input, input_length as usize) };

        let ctx = v.secret_import(secret_data, atts)?;
        Ok(SECRETS.insert_object(ctx)?)
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
        let ctx = SECRETS.get_object(secret)?;
        let key = v.secret_export(ctx.as_ref())?;
        if output_buffer_size < key.as_ref().len() as u32 {
            return Err(Error::BufferTooSmall.into());
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
        let ctx = SECRETS.get_object(secret)?;
        let key = v.secret_public_key_get(&ctx)?;
        if output_buffer_size < key.as_ref().len() as u32 {
            return Err(Error::BufferTooSmall.into());
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
        let ctx = SECRETS.get_object(secret)?;
        let atts = v.secret_attributes_get(ctx.as_ref())?;
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
        let ctx = SECRETS
            .remove_object(secret)
            .or(Err(Error::EntryNotFound))?;
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
        let ctx = SECRETS.get_object(secret)?;
        let atts = v.secret_attributes_get(&ctx)?;
        let pubkey = match atts.stype {
            SecretType::Curve25519 => {
                if peer_publickey.len() != 32 {
                    Err(Error::InvalidPublicKey)
                } else {
                    Ok(PublicKey::new(peer_publickey.to_vec()))
                }
            }
            SecretType::P256 => {
                if peer_publickey.len() != 65 {
                    Err(Error::InvalidPublicKey)
                } else {
                    Ok(PublicKey::new(peer_publickey.to_vec()))
                }
            }
            _ => Err(Error::UnknownPublicKeyType),
        }?;
        let shared_ctx = v.ec_diffie_hellman(&ctx, pubkey.as_ref())?;
        Ok(SECRETS.insert_object(shared_ctx)?)
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
        let salt_ctx = SECRETS.get_object(salt)?;
        let ikm_ctx = if input_key_material.is_null() {
            None
        } else {
            let ctx = SECRETS.get_object(unsafe { *input_key_material })?;
            Some(ctx)
        };
        let ikm_ctx = ikm_ctx.as_deref();
        let array: &[FfiSecretAttributes] =
            unsafe { slice::from_raw_parts(derived_outputs_attributes, derived_outputs_count) };

        let output_attributes: Vec<SecretAttributes> = array.iter().map(|x| x.into()).collect();

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
        let hkdf_output: Result<Vec<SecretKeyHandle>, FfiOckamError> = v
            .hkdf_sha256(&salt_ctx, b"", ikm_ctx, output_attributes)?
            .into_iter()
            .map(|x| SECRETS.insert_object(x))
            .collect();
        let hkdf_output = hkdf_output?;

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
        let ctx = SECRETS.get_object(secret)?;
        let mut nonce_vec = vec![0; 12 - 2];
        nonce_vec.extend_from_slice(&nonce.to_be_bytes());
        let ciphertext = v.aead_aes_gcm_encrypt(&ctx, plaintext, &nonce_vec, additional_data)?;

        if ciphertext_and_tag_size < ciphertext.len() as u32 {
            return Err(Error::BufferTooSmall.into());
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
        let ctx = SECRETS.get_object(secret)?;
        let mut nonce_vec = vec![0; 12 - 2];
        nonce_vec.extend_from_slice(&nonce.to_be_bytes());
        let plain =
            v.aead_aes_gcm_decrypt(&ctx, ciphertext_and_tag, &nonce_vec, additional_data)?;
        if plaintext_size < plain.len() as u32 {
            return Err(Error::BufferTooSmall.into());
        }
        *plaintext_length = plain.len() as u32;
        unsafe { std::ptr::copy_nonoverlapping(plain.as_ptr(), plaintext, plain.len()) };
        Ok(())
    }) {
        Ok(_) => FfiOckamError::none(),
        Err(err) => err.into(),
    }
}

#[no_mangle]
pub extern "C" fn ockam_vault_get_persistence_id(
    context: FfiVaultFatPointer,
    secret: SecretKeyHandle,
    persistence_id: *mut c_char,
    persistence_id_size: u32,
) -> FfiOckamError {
    match call(context, |v| -> Result<(), FfiOckamError> {
        let ctx = SECRETS.get_object(secret)?;
        let persistence_id_str = v.get_persistence_id(&ctx)?;
        let persistence_id_str =
            CString::new(persistence_id_str).map_err(|_| Error::InvalidString)?;
        let persistence_id_str = persistence_id_str.as_bytes_with_nul();
        if persistence_id_size < persistence_id_str.len() as u32 {
            return Err(Error::BufferTooSmall.into());
        }
        unsafe {
            std::ptr::copy_nonoverlapping(
                persistence_id_str.as_ptr(),
                persistence_id as *mut u8,
                persistence_id_str.len(),
            )
        };

        Ok(())
    }) {
        Ok(_) => FfiOckamError::none(),
        Err(err) => err.into(),
    }
}

#[no_mangle]
pub extern "C" fn ockam_vault_get_persistent_secret(
    context: FfiVaultFatPointer,
    secret: &mut SecretKeyHandle,
    persistence_id: *mut c_char,
) -> FfiOckamError {
    *secret = match call(context, |v| -> Result<u64, FfiOckamError> {
        let persistence_id = unsafe { CStr::from_ptr(persistence_id) }
            .to_str()
            .map_err(|_| Error::InvalidString)?;
        let ctx = v.get_persistent_secret(persistence_id)?;
        Ok(SECRETS.insert_object(ctx)?)
    }) {
        Ok(s) => s,
        Err(err) => return err.into(),
    };

    FfiOckamError::none()
}

/// Deinitialize an Ockam vault
#[no_mangle]
pub extern "C" fn ockam_vault_deinit(context: FfiVaultFatPointer) -> FfiOckamError {
    match context.vault_type {
        FfiVaultType::Software => match DEFAULT_VAULTS.remove_object(context.handle) {
            Ok(_) => FfiOckamError::none(),
            Err(_) => Error::VaultNotFound.into(),
        },
        FfiVaultType::Filesystem => match FILESYSTEM_VAULTS.remove_object(context.handle) {
            Ok(_) => FfiOckamError::none(),
            Err(_) => Error::VaultNotFound.into(),
        },
    }
}
