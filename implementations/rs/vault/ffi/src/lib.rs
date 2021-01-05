use crate::default_vault_adapter::DefaultVaultAdapter;
use crate::error::Error;
use crate::types::*;
use ffi_support::{ConcurrentHandleMap, ErrorCode, ExternError, FfiStr};
use ockam_common::error::OckamResult;
use ockam_vault_file::FilesystemVault;
use ockam_vault_software::ockam_vault::types::{PublicKey, SecretAttributes, SecretType};
use ockam_vault_software::ockam_vault::{
    AsymmetricVault, HashVault, PersistentVault, Secret, SecretVault, SymmetricVault,
};
use ockam_vault_software::DefaultVault;
use std::collections::BTreeMap;
use std::ffi::{CStr, CString};
use std::ops::DerefMut;
use std::os::raw::c_char;
use std::panic::AssertUnwindSafe;
use std::slice;

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate ffi_support;

#[macro_use]
mod macros;
mod default_vault_adapter;
pub mod error;
mod types;

trait FfiVault: SecretVault + HashVault + SymmetricVault + AsymmetricVault + PersistentVault {}

impl<D> FfiVault for D where
    D: SecretVault + HashVault + SymmetricVault + AsymmetricVault + PersistentVault
{
}

/// Wraps a vault that can be used as a trait object
struct BoxVault {
    vault: Box<dyn FfiVault + Send>,
    map: BTreeMap<SecretKeyHandle, Box<dyn Secret>>,
    next_id: SecretKeyHandle,
}

impl BoxVault {
    fn add_secret(&mut self, secret: Box<dyn Secret>) -> SecretKeyHandle {
        self.next_id += 1;
        self.map.insert(self.next_id, secret);
        self.next_id
    }

    fn get_secret(
        map: &BTreeMap<SecretKeyHandle, Box<dyn Secret>>,
        secret: SecretKeyHandle,
    ) -> OckamResult<&Box<dyn Secret>> {
        map.get(&secret).ok_or(Error::EntryNotFound.into())
    }
}

lazy_static! {
    static ref VAULTS: ConcurrentHandleMap<BoxVault> = ConcurrentHandleMap::new();
}

/// Create a new Ockam Default vault and return it
#[no_mangle]
pub extern "C" fn ockam_vault_default_init(context: &mut u64, error: &mut ExternError) {
    // TODO: handle logging
    *context = VAULTS.insert_with_output(error, || BoxVault {
        vault: Box::new(DefaultVaultAdapter::new(DefaultVault::default())),
        map: BTreeMap::new(),
        next_id: 0,
    });
}

/// Create a new Ockam file vault and return it
#[no_mangle]
pub extern "C" fn ockam_vault_file_init(
    context: &mut u64,
    path: FfiStr<'_>,
    error: &mut ExternError,
) {
    let path = path.into_string();
    *context = VAULTS.insert_with_result(error, || {
        match FilesystemVault::new(std::path::PathBuf::from(path)) {
            Ok(v) => Ok(BoxVault {
                vault: Box::new(v),
                map: BTreeMap::new(),
                next_id: 0,
            }),
            Err(_) => Err(ExternError::new_error(ErrorCode::new(1), "")),
        }
    });
}

/// Compute the SHA-256 hash on `input` and put the result in `digest`.
/// `digest` must be 32 bytes in length
#[no_mangle]
pub extern "C" fn ockam_vault_sha256(
    context: u64,
    input: *const u8,
    input_length: u32,
    digest: *mut u8,
    error: &mut ExternError,
) {
    check_buffer!(input, error);
    check_buffer!(digest, error);

    let input = unsafe { std::slice::from_raw_parts(input, input_length as usize) };

    VAULTS.call_with_result(error, context, |v| -> Result<(), ExternError> {
        let d = v.vault.sha256(input)?;
        unsafe {
            std::ptr::copy_nonoverlapping(d.as_ptr(), digest, d.len());
        }

        Ok(())
    });
}

/// Generate a secret key with the specific attributes.
/// Returns a handle for the secret
#[no_mangle]
pub extern "C" fn ockam_vault_secret_generate(
    context: u64,
    secret: &mut SecretKeyHandle,
    attributes: FfiSecretAttributes,
    error: &mut ExternError,
) {
    let atts = attributes.into();
    *secret = VAULTS.call_with_result_mut(
        error,
        context,
        |v| -> Result<SecretKeyHandle, ExternError> {
            let ctx = v.vault.secret_generate(atts)?;
            Ok(v.add_secret(ctx))
        },
    );
}

/// Import a secret key with the specific handle and attributes
#[no_mangle]
pub extern "C" fn ockam_vault_secret_import(
    context: u64,
    secret: &mut SecretKeyHandle,
    attributes: FfiSecretAttributes,
    input: *mut u8,
    input_length: u32,
    error: &mut ExternError,
) {
    check_buffer!(input, input_length, error);

    let atts = attributes.into();
    *secret = VAULTS.call_with_result_mut(
        error,
        context,
        move |v| -> Result<SecretKeyHandle, ExternError> {
            let secret_data = unsafe { std::slice::from_raw_parts(input, input_length as usize) };

            let ctx = v.vault.secret_import(secret_data, atts)?;
            Ok(v.add_secret(ctx))
        },
    );
}

/// Export a secret key with the specific handle to the output buffer
#[no_mangle]
pub extern "C" fn ockam_vault_secret_export(
    context: u64,
    secret: SecretKeyHandle,
    output_buffer: &mut u8,
    output_buffer_size: u32,
    output_buffer_length: &mut u32,
    error: &mut ExternError,
) {
    *output_buffer_length = 0;
    let mut output_buffer = AssertUnwindSafe(output_buffer);
    let mut output_buffer_length = AssertUnwindSafe(output_buffer_length);
    VAULTS.call_with_result_mut(error, context, move |v| -> Result<(), ExternError> {
        let ctx = BoxVault::get_secret(&v.map, secret)?;
        let key = v.vault.secret_export(&ctx)?;
        if output_buffer_size < key.as_ref().len() as u32 {
            return Err(Error::BufferTooSmall.into());
        }
        **output_buffer_length = key.as_ref().len() as u32;
        unsafe {
            std::ptr::copy_nonoverlapping(
                key.as_ref().as_ptr(),
                *output_buffer.deref_mut(),
                key.as_ref().len(),
            );
        };
        Ok(())
    });
}

/// Get the public key from a secret key to the output buffer
#[no_mangle]
pub extern "C" fn ockam_vault_secret_publickey_get(
    context: u64,
    secret: SecretKeyHandle,
    output_buffer: &mut u8,
    output_buffer_size: u32,
    output_buffer_length: &mut u32,
    error: &mut ExternError,
) {
    *output_buffer_length = 0;
    let mut output_buffer = AssertUnwindSafe(output_buffer);
    let mut output_buffer_length = AssertUnwindSafe(output_buffer_length);
    VAULTS.call_with_result_mut(error, context, move |v| -> Result<(), ExternError> {
        let ctx = BoxVault::get_secret(&v.map, secret)?;
        let key = v.vault.secret_public_key_get(&ctx)?;
        if output_buffer_size < key.as_ref().len() as u32 {
            return Err(Error::BufferTooSmall.into());
        }
        **output_buffer_length = key.as_ref().len() as u32;
        unsafe {
            std::ptr::copy_nonoverlapping(
                key.as_ref().as_ptr(),
                *output_buffer.deref_mut(),
                key.as_ref().len(),
            );
        };
        Ok(())
    });
}

/// Retrieve the attributes for a specified secret
#[no_mangle]
pub extern "C" fn ockam_vault_secret_attributes_get(
    context: u64,
    secret_handle: SecretKeyHandle,
    attributes: &mut FfiSecretAttributes,
    error: &mut ExternError,
) {
    let mut attributes = AssertUnwindSafe(attributes);
    VAULTS.call_with_result_mut(error, context, move |v| -> Result<(), ExternError> {
        let ctx = BoxVault::get_secret(&v.map, secret_handle)?;
        let atts = v.vault.secret_attributes_get(ctx)?;
        **attributes = atts.into();
        Ok(())
    });
}

/// Delete an ockam vault secret.
#[no_mangle]
pub extern "C" fn ockam_vault_secret_destroy(
    context: u64,
    secret: SecretKeyHandle,
    error: &mut ExternError,
) {
    VAULTS.call_with_result_mut(error, context, |v| -> Result<(), ExternError> {
        let ctx = v.map.remove(&secret).ok_or(Error::EntryNotFound)?;
        v.vault.secret_destroy(ctx)?;
        Ok(())
    });
}

/// Perform an ECDH operation on the supplied ockam vault secret and peer_publickey. The result is
/// another ockam vault secret of type unknown.
#[no_mangle]
pub extern "C" fn ockam_vault_ecdh(
    context: u64,
    secret: SecretKeyHandle,
    peer_publickey: *const u8,
    peer_publickey_length: u32,
    shared_secret: &mut SecretKeyHandle,
    error: &mut ExternError,
) {
    check_buffer!(peer_publickey, peer_publickey_length, error);
    let peer_publickey =
        unsafe { std::slice::from_raw_parts(peer_publickey, peer_publickey_length as usize) };
    let mut shared_secret = AssertUnwindSafe(shared_secret);
    VAULTS.call_with_result_mut(error, context, move |v| -> Result<(), ExternError> {
        let ctx = BoxVault::get_secret(&v.map, secret)?;
        let atts = v.vault.secret_attributes_get(&ctx)?;
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
        let shared_ctx = v.vault.ec_diffie_hellman(&ctx, pubkey.as_ref())?;
        **shared_secret = v.add_secret(shared_ctx);
        Ok(())
    });
}

/// Perform an HMAC-SHA256 based key derivation function on the supplied salt and input key
/// material.
#[no_mangle]
pub extern "C" fn ockam_vault_hkdf_sha256(
    context: u64,
    salt: SecretKeyHandle,
    input_key_material: *const SecretKeyHandle,
    derived_outputs_attributes: *const FfiSecretAttributes,
    derived_outputs_count: u8,
    derived_outputs: *mut SecretKeyHandle,
    error: &mut ExternError,
) {
    let derived_outputs_count = derived_outputs_count as usize;
    let derived_outputs = AssertUnwindSafe(derived_outputs);
    VAULTS.call_with_result_mut(error, context, move |v| -> Result<(), ExternError> {
        let salt_ctx = BoxVault::get_secret(&v.map, salt)?;
        let ikm_ctx = if input_key_material.is_null() {
            None
        } else {
            unsafe { Some(BoxVault::get_secret(&v.map, *input_key_material)?) }
        };

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
        let hkdf_output: Vec<SecretKeyHandle> = v
            .vault
            .hkdf_sha256(&salt_ctx, b"", ikm_ctx, output_attributes)?
            .into_iter()
            .map(|x| v.add_secret(x))
            .collect();

        unsafe {
            std::ptr::copy_nonoverlapping(
                hkdf_output.as_ptr(),
                *derived_outputs,
                derived_outputs_count,
            )
        };
        Ok(())
    });
}

///   Encrypt a payload using AES-GCM.
#[no_mangle]
pub extern "C" fn ockam_vault_aead_aes_gcm_encrypt(
    context: u64,
    secret: SecretKeyHandle,
    nonce: u16,
    additional_data: *const u8,
    additional_data_length: u32,
    plaintext: *const u8,
    plaintext_length: u32,
    ciphertext_and_tag: &mut u8,
    ciphertext_and_tag_size: u32,
    ciphertext_and_tag_length: &mut u32,
    error: &mut ExternError,
) {
    check_buffer!(additional_data, error);
    check_buffer!(plaintext, error);
    *ciphertext_and_tag_length = 0;
    let additional_data =
        unsafe { std::slice::from_raw_parts(additional_data, additional_data_length as usize) };
    let plaintext = unsafe { std::slice::from_raw_parts(plaintext, plaintext_length as usize) };
    let mut ciphertext_and_tag = AssertUnwindSafe(ciphertext_and_tag);
    let mut ciphertext_and_tag_length = AssertUnwindSafe(ciphertext_and_tag_length);
    VAULTS.call_with_result_mut(error, context, move |v| -> Result<(), ExternError> {
        let ctx = BoxVault::get_secret(&v.map, secret)?;
        let mut nonce_vec = vec![0; 12 - 2];
        nonce_vec.extend_from_slice(&nonce.to_be_bytes());
        let ciphertext =
            v.vault
                .aead_aes_gcm_encrypt(&ctx, plaintext, &nonce_vec, additional_data)?;

        if ciphertext_and_tag_size < ciphertext.len() as u32 {
            return Err(Error::BufferTooSmall.into());
        }
        **ciphertext_and_tag_length = ciphertext.len() as u32;
        unsafe {
            std::ptr::copy_nonoverlapping(
                ciphertext.as_ptr(),
                *ciphertext_and_tag.deref_mut(),
                ciphertext.len(),
            )
        };
        Ok(())
    });
}

/// Decrypt a payload using AES-GCM.
#[no_mangle]
pub extern "C" fn ockam_vault_aead_aes_gcm_decrypt(
    context: u64,
    secret: SecretKeyHandle,
    nonce: u16,
    additional_data: *const u8,
    additional_data_length: u32,
    ciphertext_and_tag: *const u8,
    ciphertext_and_tag_length: u32,
    plaintext: &mut u8,
    plaintext_size: u32,
    plaintext_length: &mut u32,
    error: &mut ExternError,
) {
    check_buffer!(ciphertext_and_tag, ciphertext_and_tag_length, error);
    check_buffer!(additional_data, error);
    *plaintext_length = 0;
    let additional_data =
        unsafe { std::slice::from_raw_parts(additional_data, additional_data_length as usize) };
    let ciphertext_and_tag = unsafe {
        std::slice::from_raw_parts(ciphertext_and_tag, ciphertext_and_tag_length as usize)
    };
    let mut plaintext = AssertUnwindSafe(plaintext);
    let mut plaintext_length = AssertUnwindSafe(plaintext_length);
    VAULTS.call_with_result_mut(error, context, move |v| -> Result<(), ExternError> {
        let ctx = BoxVault::get_secret(&v.map, secret)?;
        let mut nonce_vec = vec![0; 12 - 2];
        nonce_vec.extend_from_slice(&nonce.to_be_bytes());
        let plain =
            v.vault
                .aead_aes_gcm_decrypt(&ctx, ciphertext_and_tag, &nonce_vec, additional_data)?;
        if plaintext_size < plain.len() as u32 {
            return Err(Error::BufferTooSmall.into());
        }
        **plaintext_length = plain.len() as u32;
        unsafe {
            std::ptr::copy_nonoverlapping(plain.as_ptr(), *plaintext.deref_mut(), plain.len())
        };
        Ok(())
    });
}

#[no_mangle]
pub extern "C" fn ockam_vault_get_persistence_id(
    context: u64,
    secret: SecretKeyHandle,
    persistence_id: *mut c_char,
    persistence_id_size: u32,
    error: &mut ExternError,
) {
    VAULTS.call_with_result_mut(error, context, move |v| -> Result<(), ExternError> {
        let ctx = BoxVault::get_secret(&v.map, secret)?;
        let persistence_id_str = v.vault.get_persistence_id(&ctx)?;
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
    });
}

#[no_mangle]
pub extern "C" fn ockam_vault_get_persistent_secret(
    context: u64,
    secret: &mut SecretKeyHandle,
    persistence_id: *mut c_char,
    error: &mut ExternError,
) {
    let mut secret = AssertUnwindSafe(secret);
    VAULTS.call_with_result_mut(error, context, move |v| -> Result<(), ExternError> {
        let persistence_id = unsafe { CStr::from_ptr(persistence_id) }
            .to_str()
            .map_err(|_| Error::InvalidString)?;
        let ctx = v.vault.get_persistent_secret(persistence_id)?;
        **secret = v.add_secret(ctx);
        Ok(())
    });
}

/// Deinitialize an Ockam vault
#[no_mangle]
pub extern "C" fn ockam_vault_deinit(context: u64, error: &mut ExternError) {
    *error = match VAULTS.remove_u64(context) {
        Ok(_) => ExternError::success(),
        Err(_) => Error::VaultNotFound.into(),
    };
}

define_string_destructor!(string_free);
