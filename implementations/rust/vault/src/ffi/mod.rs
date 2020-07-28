use crate::types::{PublicKey, SecretKeyContext};
use crate::{
    error::*,
    ffi::types::*,
    software::DefaultVault,
    types::{SecretKeyType, SecretPersistenceType, SecretPurposeType},
    Vault,
};
use ffi_support::{ByteBuffer, ConcurrentHandleMap, ExternError, IntoFfi};
use std::{convert::TryInto, ffi::CStr, str::FromStr};

mod types;

/// A context object to interface with C
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct OckamVaultContext {
    handle: VaultHandle,
    vault_id: VaultId,
}

/// A context object for using secrets in vaults
#[repr(C)]
pub struct OckamSecret {
    attributes: FfiSecretKeyAttributes,
    handle: SecretKeyHandle,
}

impl OckamSecret {
    /// Get the string handle represented by this Secret
    pub fn get_handle(&self) -> String {
        if self.handle.is_null() {
            String::new()
        } else {
            unsafe { CStr::from_ptr(self.handle) }
                .to_string_lossy()
                .to_string()
        }
    }
}

/// Represents a Vault id
pub type VaultId = u32;
/// Represents a Vault handle
pub type VaultHandle = u64;
/// Represents a Vault error code
pub type VaultError = u32;
///
pub type SecretKeyHandle = *mut std::os::raw::c_char;
/// No error or success
pub const ERROR_NONE: u32 = 0;

lazy_static! {
    static ref DEFAULT_VAULTS: ConcurrentHandleMap<DefaultVault> = ConcurrentHandleMap::new();
}

/// The Default vault id across the FFI boundary
pub const DEFAULT_VAULT_ID: VaultId = 1;

/// Create a new Ockam Default vault and return it
#[no_mangle]
pub extern "C" fn ockam_vault_default_init(context: &mut OckamVaultContext) -> VaultError {
    let mut err = ExternError::success();
    // TODO: handle logging
    let handle = DEFAULT_VAULTS.insert_with_output(&mut err, DefaultVault::default);
    *context = OckamVaultContext {
        handle,
        vault_id: DEFAULT_VAULT_ID,
    };
    ERROR_NONE
}

/// Fill a preallocated buffer with random data.
/// Can still cause memory seg fault if `buffer` doesn't have enough space to match
/// `buffer_len`. Unfortunately, there is no way to check for this.
#[no_mangle]
pub extern "C" fn ockam_vault_random_bytes_generate(
    context: OckamVaultContext,
    buffer: *mut u8,
    buffer_len: u32,
) -> VaultError {
    check_buffer!(buffer, buffer_len);

    let mut err = ExternError::success();
    match context.vault_id {
        DEFAULT_VAULT_ID => {
            let output = DEFAULT_VAULTS.call_with_result_mut(
                &mut err,
                context.handle,
                |vault| -> Result<ByteBuffer, VaultFailError> {
                    let mut data = vec![0u8; buffer_len as usize];
                    vault.random(data.as_mut_slice())?;
                    let byte_buffer = ByteBuffer::from_vec(data);
                    Ok(byte_buffer)
                },
            );
            if err.get_code().is_success() {
                let output = output.into_vec();
                unsafe {
                    std::ptr::copy_nonoverlapping(output.as_ptr(), buffer, buffer_len as usize);
                }
                ERROR_NONE
            } else {
                VaultFailErrorKind::Random.into()
            }
        }
        _ => VaultFailErrorKind::InvalidContext.into(),
    }
}

/// Compute the SHA-256 hash on `input` and put the result in `digest`.
/// `digest` must be 32 bytes in length
#[no_mangle]
pub extern "C" fn ockam_vault_sha256(
    context: OckamVaultContext,
    input: *const u8,
    input_length: u32,
    digest: *mut u8,
) -> VaultError {
    check_buffer!(input, input_length);
    check_buffer!(digest);

    let input = unsafe { std::slice::from_raw_parts(input, input_length as usize) };

    let mut err = ExternError::success();
    match context.vault_id {
        DEFAULT_VAULT_ID => {
            let output = DEFAULT_VAULTS.call_with_result(
                &mut err,
                context.handle,
                |vault| -> Result<ByteBuffer, VaultFailError> {
                    let digest = vault.sha256(input)?;
                    let byte_buffer = ByteBuffer::from_vec(digest.to_vec());
                    Ok(byte_buffer)
                },
            );
            if err.get_code().is_success() {
                let output = output.into_vec();
                unsafe {
                    std::ptr::copy_nonoverlapping(output.as_ptr(), digest, 32);
                }
                ERROR_NONE
            } else {
                VaultFailErrorKind::Sha256.into()
            }
        }
        _ => VaultFailErrorKind::InvalidContext.into(),
    }
}

/// Generate a secret key with the specific attributes.
/// Returns a handle for the secret
#[no_mangle]
pub extern "C" fn ockam_vault_secret_generate(
    context: OckamVaultContext,
    secret: &mut OckamSecret,
    attributes: FfiSecretKeyAttributes,
) -> VaultError {
    let mut err = ExternError::success();
    let atts = attributes.into();
    match context.vault_id {
        DEFAULT_VAULT_ID => {
            let handle = DEFAULT_VAULTS.call_with_result_mut(
                &mut err,
                context.handle,
                |vault| -> Result<SecretKeyHandle, VaultFailError> {
                    let ctx = vault.secret_generate(atts)?;
                    Ok(ctx.into_ffi_value())
                },
            );
            if err.get_code().is_success() {
                *secret = OckamSecret { attributes, handle };
                ERROR_NONE
            } else {
                VaultFailErrorKind::SecretGenerate.into()
            }
        }
        _ => VaultFailErrorKind::InvalidContext.into(),
    }
}

/// Import a secret key with the specific handle and attributes
#[no_mangle]
pub extern "C" fn ockam_vault_secret_import(
    context: OckamVaultContext,
    secret: &mut OckamSecret,
    attributes: FfiSecretKeyAttributes,
    input: *mut u8,
    input_length: u32,
) -> VaultError {
    let mut err = ExternError::success();
    let atts = attributes.into();
    match context.vault_id {
        DEFAULT_VAULT_ID => {
            let handle = DEFAULT_VAULTS.call_with_result_mut(
                &mut err,
                context.handle,
                |vault| -> Result<SecretKeyHandle, VaultFailError> {
                    let ffi_sk = FfiSecretKey {
                        xtype: attributes.xtype,
                        length: input_length,
                        buffer: input,
                    };

                    let sk: crate::SecretKey = ffi_sk.try_into()?;
                    let ctx = vault.secret_import(&sk, atts)?;
                    Ok(ctx.into_ffi_value())
                },
            );
            if err.get_code().is_success() {
                *secret = OckamSecret { attributes, handle };
                ERROR_NONE
            } else {
                VaultFailErrorKind::InvalidSecret.into()
            }
        }
        _ => VaultFailErrorKind::InvalidContext.into(),
    }
}

/// Export a secret key with the specific handle to the output buffer
#[no_mangle]
pub extern "C" fn ockam_vault_secret_export(
    context: OckamVaultContext,
    secret: OckamSecret,
    output_buffer: &mut u8,
    output_buffer_size: u32,
    output_buffer_length: &mut u32,
) -> VaultError {
    *output_buffer_length = 0;
    let mut err = ExternError::success();
    match context.vault_id {
        DEFAULT_VAULT_ID => {
            let output = DEFAULT_VAULTS.call_with_result_mut(
                &mut err,
                context.handle,
                |vault| -> Result<ByteBuffer, VaultFailError> {
                    let id_str = secret.get_handle();
                    let id = usize::from_str(&id_str)?;
                    let ctx = SecretKeyContext::Memory(id);
                    let key = vault.secret_export(ctx)?;
                    Ok(ByteBuffer::from_vec(key.as_ref().to_vec()))
                },
            );
            if err.get_code().is_success() {
                let buffer = output.into_vec();
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
        _ => VaultFailErrorKind::InvalidContext.into(),
    }
}

/// Get the public key from a secret key to the output buffer
#[no_mangle]
pub extern "C" fn ockam_vault_secret_publickey_get(
    context: OckamVaultContext,
    secret: OckamSecret,
    output_buffer: &mut u8,
    output_buffer_size: u32,
    output_buffer_length: &mut u32,
) -> VaultError {
    *output_buffer_length = 0;
    let mut err = ExternError::success();
    match context.vault_id {
        DEFAULT_VAULT_ID => {
            let output = DEFAULT_VAULTS.call_with_result_mut(
                &mut err,
                context.handle,
                |vault| -> Result<ByteBuffer, VaultFailError> {
                    let id_str = secret.get_handle();
                    let id = usize::from_str(&id_str)?;
                    let ctx = SecretKeyContext::Memory(id);
                    let key = vault.secret_public_key_get(ctx)?;
                    Ok(ByteBuffer::from_vec(key.as_ref().to_vec()))
                },
            );
            if err.get_code().is_success() {
                let buffer = output.into_vec();
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
        _ => VaultFailErrorKind::InvalidContext.into(),
    }
}

/// Retrieve the attributes for a specified secret
#[no_mangle]
pub extern "C" fn ockam_vault_secret_attributes_get(
    context: OckamVaultContext,
    secret: OckamSecret,
    attributes: &mut FfiSecretKeyAttributes,
) -> VaultError {
    let mut err = ExternError::success();
    match context.vault_id {
        DEFAULT_VAULT_ID => {
            let output = DEFAULT_VAULTS.call_with_result_mut(
                &mut err,
                context.handle,
                |vault| -> Result<FfiSecretKeyAttributes, VaultFailError> {
                    let ctx = get_memory_id(secret)?;
                    let atts = vault.secret_attributes_get(ctx)?;
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
        _ => VaultFailErrorKind::InvalidContext.into(),
    }
}

/// Delete an ockam vault secret.
#[no_mangle]
pub extern "C" fn ockam_vault_secret_destroy(
    context: OckamVaultContext,
    secret: OckamSecret,
) -> VaultError {
    let mut err = ExternError::success();
    match context.vault_id {
        DEFAULT_VAULT_ID => {
            DEFAULT_VAULTS.call_with_result_mut(
                &mut err,
                context.handle,
                |vault| -> Result<(), VaultFailError> {
                    let ctx = get_memory_id(secret)?;
                    vault.secret_destroy(ctx)?;
                    Ok(())
                },
            );
            if err.get_code().is_success() {
                ERROR_NONE
            } else {
                VaultFailErrorKind::GetAttributes.into()
            }
        }
        _ => VaultFailErrorKind::InvalidContext.into(),
    }
}

/// Perform an ECDH operation on the supplied ockam vault secret and peer_publickey. The result is
/// another ockam vault secret of type unknown.
#[no_mangle]
pub extern "C" fn ockam_vault_ecdh(
    context: OckamVaultContext,
    secret: OckamSecret,
    peer_publickey: *const u8,
    peer_publickey_length: u32,
    shared_secret: &mut OckamSecret,
) -> VaultError {
    check_buffer!(peer_publickey, peer_publickey_length);
    let mut err = ExternError::success();
    let peer_publickey =
        unsafe { std::slice::from_raw_parts(peer_publickey, peer_publickey_length as usize) };
    match context.vault_id {
        DEFAULT_VAULT_ID => {
            let handle = DEFAULT_VAULTS.call_with_result_mut(
                &mut err,
                context.handle,
                |vault| -> Result<SecretKeyHandle, VaultFailError> {
                    let ctx = get_memory_id(secret)?;
                    let atts = vault.secret_attributes_get(ctx)?;
                    let pubkey = match atts.xtype {
                        SecretKeyType::Curve25519 => {
                            if peer_publickey.len() != 32 {
                                Err(VaultFailErrorKind::Ecdh.into())
                            } else {
                                Ok(PublicKey::Curve25519(*array_ref![peer_publickey, 0, 32]))
                            }
                        }
                        SecretKeyType::P256 => {
                            if peer_publickey.len() != 65 {
                                Err(VaultFailErrorKind::Ecdh.into())
                            } else {
                                Ok(PublicKey::P256(*array_ref![peer_publickey, 0, 65]))
                            }
                        }
                        _ => Err(VaultFailError::from(VaultFailErrorKind::Ecdh)),
                    }?;
                    let shared_ctx = vault.ec_diffie_hellman(ctx, pubkey)?;
                    Ok(shared_ctx.into_ffi_value())
                },
            );
            if err.get_code().is_success() {
                let attributes = FfiSecretKeyAttributes {
                    xtype: SecretKeyType::Buffer(0).to_usize() as u32,
                    purpose: SecretPurposeType::KeyAgreement.to_usize() as u32,
                    persistence: SecretPersistenceType::Ephemeral.to_usize() as u32
                };
                *shared_secret = OckamSecret { attributes, handle };
                ERROR_NONE
            } else {
                VaultFailErrorKind::Ecdh.into()
            }
        }
        _ => VaultFailErrorKind::InvalidContext.into(),
    }
}


///   Encrypt a payload using AES-GCM.
#[no_mangle]
pub extern "C" fn ockam_vault_aead_aes_gcm_encrypt(context: OckamVaultContext,
                                                   secret: OckamSecret,
                                                   nonce: u16,
                                                   additional_data: *const u8,
                                                   additional_data_length: u32,
                                                   plaintext: *const u8,
                                                   plaintext_length: u32,
                                                   ciphertext_and_tag: &mut u8,
                                                   ciphertext_and_tag_size: u32,
                                                   ciphertext_and_tag_length: &mut u32) -> VaultError {
    check_buffer!(additional_data, additional_data_length);
    check_buffer!(plaintext, plaintext_length);
    *ciphertext_and_tag_length = 0;
    let mut err = ExternError::success();
    let additional_data = unsafe { std::slice::from_raw_parts(additional_data, additional_data_length as usize) };
    let plaintext = unsafe { std::slice::from_raw_parts(plaintext, plaintext_length as usize) };
    match context.vault_id {
        DEFAULT_VAULT_ID => {
            let output = DEFAULT_VAULTS.call_with_result_mut(
                &mut err,
                context.handle,
                |vault| -> Result<ByteBuffer, VaultFailError> {
                    let ctx = get_memory_id(secret)?;
                    let ciphertext = vault.aead_aes_gcm_encrypt(ctx, plaintext, nonce.to_be_bytes().as_ref(), additional_data)?;
                    Ok(ByteBuffer::from_vec(ciphertext))
                },
            );
            if err.get_code().is_success() {
                let buffer = output.into_vec();
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
        _ => VaultFailErrorKind::InvalidContext.into(),
    }
}

/// Decrypt a payload using AES-GCM.
pub extern "C" fn ockam_vault_aead_aes_gcm_decrypt(context: OckamVaultContext,
                                                   secret: OckamSecret,
                                                   nonce: u16,
                                                   additional_data: *const u8,
                                                   additional_data_length: u32,
                                                   ciphertext_and_tag: *const u8,
                                                   ciphertext_and_tag_length: u32,
                                                   plaintext: &mut u8,
                                                   plaintext_size: u32,
                                                   plaintext_length: &mut u32) -> VaultError {
    check_buffer!(additional_data, additional_data_length);
    check_buffer!(ciphertext_and_tag, plaintext_size);
    *plaintext_length = 0;
    let mut err = ExternError::success();
    let additional_data = unsafe { std::slice::from_raw_parts(additional_data, additional_data_length as usize) };
    let ciphertext_and_tag = unsafe { std::slice::from_raw_parts(ciphertext_and_tag, ciphertext_and_tag_length as usize) };
    match context.vault_id {
        DEFAULT_VAULT_ID => {
            let output = DEFAULT_VAULTS.call_with_result_mut(
                &mut err,
                context.handle,
                |vault| -> Result<ByteBuffer, VaultFailError> {
                    let ctx = get_memory_id(secret)?;
                    let plain = vault.aead_aes_gcm_decrypt(ctx, ciphertext_and_tag, nonce.to_be_bytes().as_ref(), additional_data)?;
                    Ok(ByteBuffer::from_vec(plain))
                },
            );
            if err.get_code().is_success() {
                let buffer = output.into_vec();
                if plaintext_size < buffer.len() as u32 {
                    VaultFailErrorKind::AeadAesGcmDecrypt.into()
                } else {
                    *plaintext_length = buffer.len() as u32;
                    unsafe {
                        std::ptr::copy_nonoverlapping(buffer.as_ptr(), plaintext, buffer.len())
                    };
                    ERROR_NONE
                }
            } else {
                VaultFailErrorKind::AeadAesGcmDecrypt.into()
            }
        }
        _ => VaultFailErrorKind::InvalidContext.into(),
    }
}


/// Deinitialize an Ockam vault
#[no_mangle]
pub extern "C" fn ockam_vault_deinit(context: OckamVaultContext) -> VaultError {
    let mut result: VaultError = ERROR_NONE;
    match context.vault_id {
        DEFAULT_VAULT_ID => {
            if DEFAULT_VAULTS.remove_u64(context.handle).is_err() {
                result = VaultFailErrorKind::InvalidContext.into();
            }
        }
        _ => result = VaultFailErrorKind::InvalidContext.into(),
    };
    result
}

define_string_destructor!(string_free);

fn get_memory_id(secret: OckamSecret) -> Result<SecretKeyContext, VaultFailError> {
    let id_str = secret.get_handle();
    let id = usize::from_str(&id_str)?;
    Ok(SecretKeyContext::Memory(id))
}
