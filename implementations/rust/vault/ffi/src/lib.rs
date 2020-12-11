use crate::default_vault_adapter::DefaultVaultAdapter;
use crate::error::{map_nul_error, map_vault_error};
use crate::types::*;
use ffi_support::{ByteBuffer, ConcurrentHandleMap, ErrorCode, ExternError, FfiStr, Handle};
use ockam_vault_file::FilesystemVault;
use ockam_vault_software::ockam_vault::error::{VaultFailError, VaultFailErrorKind};
use ockam_vault_software::ockam_vault::types::{PublicKey, SecretAttributes, SecretType};
use ockam_vault_software::ockam_vault::{
    AsymmetricVault, HashVault, PersistentVault, Secret, SecretVault, SymmetricVault,
};
use ockam_vault_software::DefaultVault;
use std::collections::BTreeMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::slice;

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate ffi_support;

#[macro_use]
mod macros;
mod default_vault_adapter;
mod error;
mod types;

trait FfiVault: SecretVault + HashVault + SymmetricVault + AsymmetricVault + PersistentVault {}

impl<D> FfiVault for D where
    D: SecretVault + HashVault + SymmetricVault + AsymmetricVault + PersistentVault
{
}

/// Wraps a vault that can be used as a trait object
struct BoxVault {
    vault: Box<dyn FfiVault + Send>,
    map: BTreeMap<u64, Box<dyn Secret>>,
    next_id: u64,
}

impl BoxVault {
    fn add_secret(&mut self, secret: Box<dyn Secret>) -> u64 {
        self.next_id += 1;
        self.map.insert(self.next_id, secret);
        self.next_id
    }
}

lazy_static! {
    static ref VAULTS: ConcurrentHandleMap<BoxVault> = ConcurrentHandleMap::new();
}

/// Create a new Ockam Default vault and return it
#[no_mangle]
pub extern "C" fn ockam_vault_default_init(context: &mut u64) -> VaultError {
    let mut err = ExternError::success();
    // TODO: handle logging
    *context = VAULTS.insert_with_output(&mut err, || BoxVault {
        vault: Box::new(DefaultVaultAdapter::new(DefaultVault::default())),
        map: BTreeMap::new(),
        next_id: 0,
    });
    ERROR_NONE
}

/// Create a new Ockam file vault and return it
#[no_mangle]
pub extern "C" fn ockam_vault_file_init(context: &mut u64, path: FfiStr<'_>) -> VaultError {
    let mut err = ExternError::success();
    let path = path.into_string();
    *context = VAULTS.insert_with_result(&mut err, || {
        match FilesystemVault::new(std::path::PathBuf::from(path)) {
            Ok(v) => Ok(BoxVault {
                vault: Box::new(v),
                map: BTreeMap::new(),
                next_id: 0,
            }),
            Err(_) => Err(ExternError::new_error(ErrorCode::new(1), "")),
        }
    });
    if err == ExternError::success() {
        ERROR_NONE
    } else {
        ERROR
    }
}

/// Compute the SHA-256 hash on `input` and put the result in `digest`.
/// `digest` must be 32 bytes in length
#[no_mangle]
pub extern "C" fn ockam_vault_sha256(
    context: u64,
    input: *const u8,
    input_length: u32,
    digest: *mut u8,
) -> VaultError {
    check_buffer!(input);
    check_buffer!(digest);

    let input = unsafe { std::slice::from_raw_parts(input, input_length as usize) };

    let mut err = ExternError::success();
    let output =
        VAULTS.call_with_result(&mut err, context, |v| -> Result<ByteBuffer, ExternError> {
            let d = v.vault.sha256(input).map_err(map_vault_error)?;
            let byte_buffer = ByteBuffer::from_vec(d.to_vec());
            Ok(byte_buffer)
        });
    if err.get_code().is_success() {
        let output = output.destroy_into_vec();
        unsafe {
            std::ptr::copy_nonoverlapping(output.as_ptr(), digest, 32);
        }
        ERROR_NONE
    } else {
        VaultFailErrorKind::Sha256.into()
    }
}

/// Generate a secret key with the specific attributes.
/// Returns a handle for the secret
#[no_mangle]
pub extern "C" fn ockam_vault_secret_generate(
    context: u64,
    secret: &mut u64,
    attributes: FfiSecretAttributes,
) -> VaultError {
    let mut err = ExternError::success();
    let atts = attributes.into();
    let handle = VAULTS.call_with_result_mut(
        &mut err,
        context,
        |v| -> Result<SecretKeyHandle, ExternError> {
            let ctx = v.vault.secret_generate(atts).map_err(map_vault_error)?;
            Ok(v.add_secret(ctx))
        },
    );
    if err.get_code().is_success() {
        *secret = handle;
        ERROR_NONE
    } else {
        VaultFailErrorKind::SecretGenerate.into()
    }
}

/// Import a secret key with the specific handle and attributes
#[no_mangle]
pub extern "C" fn ockam_vault_secret_import(
    context: u64,
    secret: &mut u64,
    attributes: FfiSecretAttributes,
    input: *mut u8,
    input_length: u32,
) -> VaultError {
    check_buffer!(input, input_length);

    let mut err = ExternError::success();
    let atts = attributes.into();
    let handle = VAULTS.call_with_result_mut(
        &mut err,
        context,
        |v| -> Result<SecretKeyHandle, ExternError> {
            let secret_data = unsafe { std::slice::from_raw_parts(input, input_length as usize) };

            let ctx = v
                .vault
                .secret_import(secret_data, atts)
                .map_err(map_vault_error)?;
            Ok(v.add_secret(ctx))
        },
    );
    if err.get_code().is_success() {
        *secret = handle;
        ERROR_NONE
    } else {
        VaultFailErrorKind::InvalidSecret.into()
    }
}

/// Export a secret key with the specific handle to the output buffer
#[no_mangle]
pub extern "C" fn ockam_vault_secret_export(
    context: u64,
    secret: u64,
    output_buffer: &mut u8,
    output_buffer_size: u32,
    output_buffer_length: &mut u32,
) -> VaultError {
    *output_buffer_length = 0;
    let mut err = ExternError::success();
    let output =
        VAULTS.call_with_result_mut(&mut err, context, |v| -> Result<ByteBuffer, ExternError> {
            let ctx = v
                .map
                .get(&secret)
                .ok_or(VaultFailError::from(VaultFailErrorKind::InvalidSecret))
                .map_err(map_vault_error)?;
            let key = v.vault.secret_export(&ctx).map_err(map_vault_error)?;
            Ok(ByteBuffer::from_vec(key.as_ref().to_vec()))
        });
    if err.get_code().is_success() {
        let buffer = output.destroy_into_vec();
        if output_buffer_size < buffer.len() as u32 {
            VaultFailErrorKind::Export.into()
        } else {
            *output_buffer_length = buffer.len() as u32;
            unsafe {
                std::ptr::copy_nonoverlapping(buffer.as_ptr(), output_buffer, buffer.len());
            };
            ERROR_NONE
        }
    } else {
        VaultFailErrorKind::Export.into()
    }
}

/// Get the public key from a secret key to the output buffer
#[no_mangle]
pub extern "C" fn ockam_vault_secret_publickey_get(
    context: u64,
    secret: u64,
    output_buffer: &mut u8,
    output_buffer_size: u32,
    output_buffer_length: &mut u32,
) -> VaultError {
    *output_buffer_length = 0;
    let mut err = ExternError::success();
    let output =
        VAULTS.call_with_result_mut(&mut err, context, |v| -> Result<ByteBuffer, ExternError> {
            let ctx = v
                .map
                .get(&secret)
                .ok_or(VaultFailError::from(VaultFailErrorKind::InvalidSecret))
                .map_err(map_vault_error)?;
            let key = v
                .vault
                .secret_public_key_get(&ctx)
                .map_err(map_vault_error)?;
            Ok(ByteBuffer::from_vec(key.as_ref().to_vec()))
        });
    if err.get_code().is_success() {
        let buffer = output.destroy_into_vec();
        if output_buffer_size < buffer.len() as u32 {
            VaultFailErrorKind::PublicKey.into()
        } else {
            *output_buffer_length = buffer.len() as u32;
            unsafe {
                std::ptr::copy_nonoverlapping(buffer.as_ptr(), output_buffer, buffer.len());
            };
            ERROR_NONE
        }
    } else {
        VaultFailErrorKind::PublicKey.into()
    }
}

/// Retrieve the attributes for a specified secret
#[no_mangle]
pub extern "C" fn ockam_vault_secret_attributes_get(
    context: u64,
    secret_handle: u64,
    attributes: &mut FfiSecretAttributes,
) -> VaultError {
    let mut err = ExternError::success();
    let output = VAULTS.call_with_result_mut(
        &mut err,
        context,
        |v| -> Result<FfiSecretAttributes, ExternError> {
            let ctx = v
                .map
                .get(&secret_handle)
                .ok_or(VaultFailError::from(VaultFailErrorKind::InvalidSecret))
                .map_err(map_vault_error)?;
            let atts = v
                .vault
                .secret_attributes_get(ctx)
                .map_err(map_vault_error)?;
            Ok(atts.into())
        },
    );
    if err.get_code().is_success() {
        *attributes = output;
        ERROR_NONE
    } else {
        VaultFailErrorKind::GetAttributes.into()
    }
}

/// Delete an ockam vault secret.
#[no_mangle]
pub extern "C" fn ockam_vault_secret_destroy(context: u64, secret: u64) -> VaultError {
    let mut err = ExternError::success();
    VAULTS.call_with_result_mut(&mut err, context, |v| -> Result<(), ExternError> {
        let ctx = v
            .map
            .remove(&secret)
            .ok_or(VaultFailError::from(VaultFailErrorKind::InvalidSecret))
            .map_err(map_vault_error)?;
        v.vault.secret_destroy(ctx).map_err(map_vault_error)?;
        Ok(())
    });
    if err.get_code().is_success() {
        ERROR_NONE
    } else {
        VaultFailErrorKind::GetAttributes.into()
    }
}

/// Perform an ECDH operation on the supplied ockam vault secret and peer_publickey. The result is
/// another ockam vault secret of type unknown.
#[no_mangle]
pub extern "C" fn ockam_vault_ecdh(
    context: u64,
    secret: u64,
    peer_publickey: *const u8,
    peer_publickey_length: u32,
    shared_secret: &mut u64,
) -> VaultError {
    check_buffer!(peer_publickey, peer_publickey_length);
    let mut err = ExternError::success();
    let peer_publickey =
        unsafe { std::slice::from_raw_parts(peer_publickey, peer_publickey_length as usize) };
    let handle = VAULTS.call_with_result_mut(
        &mut err,
        context,
        |v| -> Result<SecretKeyHandle, ExternError> {
            let ctx = v
                .map
                .get(&secret)
                .ok_or(VaultFailError::from(VaultFailErrorKind::InvalidSecret))
                .map_err(map_vault_error)?;
            let atts = v
                .vault
                .secret_attributes_get(&ctx)
                .map_err(map_vault_error)?;
            let pubkey = match atts.stype {
                SecretType::Curve25519 => {
                    if peer_publickey.len() != 32 {
                        Err(VaultFailErrorKind::Ecdh.into())
                    } else {
                        Ok(PublicKey::new(peer_publickey.to_vec()))
                    }
                }
                SecretType::P256 => {
                    if peer_publickey.len() != 65 {
                        Err(VaultFailErrorKind::Ecdh.into())
                    } else {
                        Ok(PublicKey::new(peer_publickey.to_vec()))
                    }
                }
                _ => Err(VaultFailError::from(VaultFailErrorKind::Ecdh)),
            }
            .map_err(map_vault_error)?;
            let shared_ctx = v
                .vault
                .ec_diffie_hellman(&ctx, pubkey.as_ref())
                .map_err(map_vault_error)?;
            Ok(v.add_secret(shared_ctx))
        },
    );
    if err.get_code().is_success() {
        *shared_secret = handle;
        ERROR_NONE
    } else {
        VaultFailErrorKind::Ecdh.into()
    }
}

/// Perform an HMAC-SHA256 based key derivation function on the supplied salt and input key
/// material.
#[no_mangle]
pub extern "C" fn ockam_vault_hkdf_sha256(
    context: u64,
    salt: u64,
    input_key_material: *const u64,
    derived_outputs_attributes: *const FfiSecretAttributes,
    derived_outputs_count: u8,
    derived_outputs: *mut u64,
) -> VaultError {
    let derived_outputs_count = derived_outputs_count as usize;
    let mut err = ExternError::success();
    let handles = VAULTS.call_with_result_mut(
        &mut err,
        context,
        |v| -> Result<OckamSecretList, ExternError> {
            let salt_ctx = v
                .map
                .get(&salt)
                .ok_or(VaultFailError::from(VaultFailErrorKind::InvalidSecret))
                .map_err(map_vault_error)?;
            let ikm_ctx = if input_key_material.is_null() {
                None
            } else {
                unsafe {
                    Some(
                        v.map
                            .get(&*input_key_material)
                            .ok_or(VaultFailError::from(VaultFailErrorKind::InvalidSecret))
                            .map_err(map_vault_error)?,
                    )
                }
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
            let hkdf_output = v
                .vault
                .hkdf_sha256(&salt_ctx, b"", ikm_ctx, output_attributes)
                .map_err(map_vault_error)?
                .into_iter()
                .map(|x| v.add_secret(x))
                .collect();

            // FIXME: Double conversion is happening here
            Ok(OckamSecretList(hkdf_output))
        },
    );
    if err.get_code().is_success() {
        unsafe {
            std::ptr::copy_nonoverlapping(handles.as_ptr(), derived_outputs, derived_outputs_count)
        };
        ERROR_NONE
    } else {
        VaultFailErrorKind::HkdfSha256.into()
    }
}

///   Encrypt a payload using AES-GCM.
#[no_mangle]
pub extern "C" fn ockam_vault_aead_aes_gcm_encrypt(
    context: u64,
    secret: u64,
    nonce: u16,
    additional_data: *const u8,
    additional_data_length: u32,
    plaintext: *const u8,
    plaintext_length: u32,
    ciphertext_and_tag: &mut u8,
    ciphertext_and_tag_size: u32,
    ciphertext_and_tag_length: &mut u32,
) -> VaultError {
    check_buffer!(additional_data);
    check_buffer!(plaintext);
    *ciphertext_and_tag_length = 0;
    let mut err = ExternError::success();
    let additional_data =
        unsafe { std::slice::from_raw_parts(additional_data, additional_data_length as usize) };
    let plaintext = unsafe { std::slice::from_raw_parts(plaintext, plaintext_length as usize) };
    let output =
        VAULTS.call_with_result_mut(&mut err, context, |v| -> Result<ByteBuffer, ExternError> {
            let ctx = v
                .map
                .get(&secret)
                .ok_or(VaultFailError::from(VaultFailErrorKind::InvalidSecret))
                .map_err(map_vault_error)?;
            let mut nonce_vec = vec![0; 12 - 2];
            nonce_vec.extend_from_slice(&nonce.to_be_bytes());
            let ciphertext = v
                .vault
                .aead_aes_gcm_encrypt(&ctx, plaintext, &nonce_vec, additional_data)
                .map_err(map_vault_error)?;
            Ok(ByteBuffer::from_vec(ciphertext))
        });
    if err.get_code().is_success() {
        let buffer = output.destroy_into_vec();
        if ciphertext_and_tag_size < buffer.len() as u32 {
            VaultFailErrorKind::AeadAesGcmEncrypt.into()
        } else {
            *ciphertext_and_tag_length = buffer.len() as u32;
            unsafe {
                std::ptr::copy_nonoverlapping(buffer.as_ptr(), ciphertext_and_tag, buffer.len())
            };
            ERROR_NONE
        }
    } else {
        VaultFailErrorKind::AeadAesGcmEncrypt.into()
    }
}

/// Decrypt a payload using AES-GCM.
#[no_mangle]
pub extern "C" fn ockam_vault_aead_aes_gcm_decrypt(
    context: u64,
    secret: u64,
    nonce: u16,
    additional_data: *const u8,
    additional_data_length: u32,
    ciphertext_and_tag: *const u8,
    ciphertext_and_tag_length: u32,
    plaintext: &mut u8,
    plaintext_size: u32,
    plaintext_length: &mut u32,
) -> VaultError {
    check_buffer!(ciphertext_and_tag, ciphertext_and_tag_length);
    check_buffer!(additional_data);
    *plaintext_length = 0;
    let mut err = ExternError::success();
    let additional_data =
        unsafe { std::slice::from_raw_parts(additional_data, additional_data_length as usize) };
    let ciphertext_and_tag = unsafe {
        std::slice::from_raw_parts(ciphertext_and_tag, ciphertext_and_tag_length as usize)
    };
    let output =
        VAULTS.call_with_result_mut(&mut err, context, |v| -> Result<ByteBuffer, ExternError> {
            let ctx = v
                .map
                .get(&secret)
                .ok_or(VaultFailError::from(VaultFailErrorKind::InvalidSecret))
                .map_err(map_vault_error)?;
            let mut nonce_vec = vec![0; 12 - 2];
            nonce_vec.extend_from_slice(&nonce.to_be_bytes());
            let plain = v
                .vault
                .aead_aes_gcm_decrypt(&ctx, ciphertext_and_tag, &nonce_vec, additional_data)
                .map_err(map_vault_error)?;
            Ok(ByteBuffer::from_vec(plain))
        });
    if err.get_code().is_success() {
        let buffer = output.destroy_into_vec();
        if plaintext_size < buffer.len() as u32 {
            VaultFailErrorKind::AeadAesGcmDecrypt.into()
        } else {
            *plaintext_length = buffer.len() as u32;
            unsafe { std::ptr::copy_nonoverlapping(buffer.as_ptr(), plaintext, buffer.len()) };
            ERROR_NONE
        }
    } else {
        VaultFailErrorKind::AeadAesGcmDecrypt.into()
    }
}

#[no_mangle]
pub extern "C" fn ockam_vault_get_persistence_id(
    context: u64,
    secret: u64,
    persistence_id: *mut c_char,
    persistence_id_size: u32,
) -> VaultError {
    let handle = match Handle::from_u64(context) {
        Ok(handle) => handle,
        Err(_) => return VaultFailErrorKind::InvalidSecret.into(),
    };

    let output = match VAULTS.get(handle, |v| -> Result<CString, ExternError> {
        let ctx = v
            .map
            .get(&secret)
            .ok_or(VaultFailError::from(VaultFailErrorKind::InvalidSecret))
            .map_err(map_vault_error)?;
        let persistence_id = v.vault.get_persistence_id(&ctx).map_err(map_vault_error)?;
        let persistence_id = CString::new(persistence_id).map_err(map_nul_error)?;
        Ok(persistence_id)
    }) {
        Ok(output) => output,
        Err(_) => return VaultFailErrorKind::InvalidSecret.into(), // TODO: Fix error propagation
    };
    let output = output.to_bytes_with_nul();
    if persistence_id_size < output.len() as u32 {
        VaultFailErrorKind::InvalidContext.into()
    } else {
        unsafe {
            std::ptr::copy_nonoverlapping(output.as_ptr(), persistence_id as *mut u8, output.len())
        };
        ERROR_NONE
    }
}

#[no_mangle]
pub extern "C" fn ockam_vault_get_persistent_secret(
    context: u64,
    secret: &mut u64,
    persistence_id: *mut c_char,
) -> VaultError {
    let persistence_id = match unsafe { CStr::from_ptr(persistence_id) }.to_str() {
        Ok(id) => id,
        Err(_) => return VaultFailErrorKind::InvalidSecret.into(),
    };

    let mut err = ExternError::success();
    let handle = VAULTS.call_with_result_mut(
        &mut err,
        context,
        |v| -> Result<SecretKeyHandle, ExternError> {
            let ctx = v
                .vault
                .get_persistent_secret(persistence_id)
                .map_err(map_vault_error)?;
            Ok(v.add_secret(ctx))
        },
    );
    if err.get_code().is_success() {
        *secret = handle;
        ERROR_NONE
    } else {
        VaultFailErrorKind::SecretGenerate.into()
    }
}

/// Deinitialize an Ockam vault
#[no_mangle]
pub extern "C" fn ockam_vault_deinit(context: u64) -> VaultError {
    let mut result: VaultError = ERROR_NONE;
    if VAULTS.remove_u64(context).is_err() {
        result = VaultFailErrorKind::InvalidContext.into();
    }
    result
}

define_string_destructor!(string_free);
