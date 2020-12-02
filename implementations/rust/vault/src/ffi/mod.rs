use crate::software::DefaultVaultSecret;
use crate::types::*;
use crate::types::{PublicKey, SecretKey, SecretKeyAttributes};
use crate::{
    error::*, ffi::types::*, file::FilesystemVault, software::DefaultVault, DynVault, Secret,
};
use ffi_support::{ByteBuffer, ConcurrentHandleMap, ErrorCode, ExternError, FfiStr};
use std::convert::TryInto;
use std::slice;

mod types;

/// Implementation for this trait is needed to add ffi support for vault implementation
pub trait SecretFfiConverter {
    /// Convert secret trait to u64
    fn secret_into_ffi(&self, secret: &Box<dyn Secret>) -> Result<u64, VaultFailError>;
    /// Create secret from u64
    fn secret_from_ffi(&self, secret_handle: u64) -> Result<Box<dyn Secret>, VaultFailError>;
}

/// SecretFfiConverter implementation for DefaultVault
struct DefaultVaultSecretFfiConverter {}

/// Default
impl Default for DefaultVaultSecretFfiConverter {
    /// Default
    fn default() -> Self {
        DefaultVaultSecretFfiConverter {}
    }
}

/// SecretFfiConverter implementation for DefaultVault
impl SecretFfiConverter for DefaultVaultSecretFfiConverter {
    fn secret_into_ffi(&self, secret: &Box<dyn Secret>) -> Result<u64, VaultFailError> {
        let secret = DefaultVaultSecret::downcast_secret(secret)?;
        Ok(secret.0 as u64)
    }

    fn secret_from_ffi(&self, secret_handle: u64) -> Result<Box<dyn Secret>, VaultFailError> {
        let secret = DefaultVaultSecret(secret_handle as usize);
        Ok(Box::new(secret))
    }
}

/// Wraps a vault that can be used as a trait object
struct BoxVault {
    vault: Box<dyn DynVault + Send>,
    secret_ffi_converter: Box<dyn SecretFfiConverter + Send>,
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
        vault: Box::new(DefaultVault::default()),
        secret_ffi_converter: Box::new(DefaultVaultSecretFfiConverter::default()),
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
                secret_ffi_converter: Box::new(DefaultVaultSecretFfiConverter::default()),
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

/// Fill a preallocated buffer with random data.
/// Can still cause memory seg fault if `buffer` doesn't have enough space to match
/// `buffer_len`. Unfortunately, there is no way to check for this.
#[no_mangle]
pub extern "C" fn ockam_vault_random_bytes_generate(
    context: u64,
    buffer: *mut u8,
    buffer_len: u32,
) -> VaultError {
    check_buffer!(buffer, buffer_len);

    let mut err = ExternError::success();

    let output = VAULTS.call_with_result_mut(
        &mut err,
        context,
        |v| -> Result<ByteBuffer, VaultFailError> {
            let mut data = vec![0u8; buffer_len as usize];
            v.vault.random(data.as_mut_slice())?;
            let byte_buffer = ByteBuffer::from_vec(data);
            Ok(byte_buffer)
        },
    );
    if err.get_code().is_success() {
        let output = output.destroy_into_vec();
        unsafe {
            std::ptr::copy_nonoverlapping(output.as_ptr(), buffer, buffer_len as usize);
        }
        ERROR_NONE
    } else {
        VaultFailErrorKind::Random.into()
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
    let output = VAULTS.call_with_result(
        &mut err,
        context,
        |v| -> Result<ByteBuffer, VaultFailError> {
            let d = v.vault.sha256(input)?;
            let byte_buffer = ByteBuffer::from_vec(d.to_vec());
            Ok(byte_buffer)
        },
    );
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
    attributes: FfiSecretKeyAttributes,
) -> VaultError {
    let mut err = ExternError::success();
    let atts = attributes.into();
    let handle = VAULTS.call_with_result_mut(
        &mut err,
        context,
        |v| -> Result<SecretKeyHandle, VaultFailError> {
            let ctx = v.vault.secret_generate(atts)?;
            let ctx = v.secret_ffi_converter.secret_into_ffi(&ctx)?;
            Ok(ctx)
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
    attributes: FfiSecretKeyAttributes,
    input: *mut u8,
    input_length: u32,
) -> VaultError {
    check_buffer!(input, input_length);

    let mut err = ExternError::success();
    let atts = attributes.into();
    let handle = VAULTS.call_with_result_mut(
        &mut err,
        context,
        |v| -> Result<SecretKeyHandle, VaultFailError> {
            let ffi_sk = FfiSecretKey {
                xtype: attributes.xtype,
                length: input_length,
                buffer: input,
            };

            let sk: SecretKey = ffi_sk.try_into()?;
            let ctx = v.vault.secret_import(&sk, atts)?;
            let ctx = v.secret_ffi_converter.secret_into_ffi(&ctx)?;
            Ok(ctx)
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
    let output = VAULTS.call_with_result_mut(
        &mut err,
        context,
        |v| -> Result<ByteBuffer, VaultFailError> {
            let ctx = v.secret_ffi_converter.secret_from_ffi(secret)?;
            let key = v.vault.secret_export(&ctx)?;
            Ok(ByteBuffer::from_vec(key.as_ref().to_vec()))
        },
    );
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
    let output = VAULTS.call_with_result_mut(
        &mut err,
        context,
        |v| -> Result<ByteBuffer, VaultFailError> {
            let ctx = v.secret_ffi_converter.secret_from_ffi(secret)?;
            let key = v.vault.secret_public_key_get(&ctx)?;
            Ok(ByteBuffer::from_vec(key.as_ref().to_vec()))
        },
    );
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
    attributes: &mut FfiSecretKeyAttributes,
) -> VaultError {
    let mut err = ExternError::success();
    let output = VAULTS.call_with_result_mut(
        &mut err,
        context,
        |v| -> Result<FfiSecretKeyAttributes, VaultFailError> {
            let ctx = v.secret_ffi_converter.secret_from_ffi(secret_handle)?;
            let atts = v.vault.secret_attributes_get(&ctx)?;
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
    VAULTS.call_with_result_mut(&mut err, context, |v| -> Result<(), VaultFailError> {
        let ctx = v.secret_ffi_converter.secret_from_ffi(secret)?;
        v.vault.secret_destroy(ctx)?;
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
        |v| -> Result<SecretKeyHandle, VaultFailError> {
            let ctx = v.secret_ffi_converter.secret_from_ffi(secret)?;
            let atts = v.vault.secret_attributes_get(&ctx)?;
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
            let shared_ctx = v.vault.ec_diffie_hellman(&ctx, pubkey)?;
            Ok(v.secret_ffi_converter.secret_into_ffi(&shared_ctx)?)
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
    derived_outputs_attributes: *const FfiSecretKeyAttributes,
    derived_outputs_count: u8,
    derived_outputs: *mut u64,
) -> VaultError {
    let derived_outputs_count = derived_outputs_count as usize;
    let mut err = ExternError::success();
    let handles = VAULTS.call_with_result_mut(
        &mut err,
        context,
        |v| -> Result<OckamSecretList, VaultFailError> {
            let salt_ctx = v.secret_ffi_converter.secret_from_ffi(salt)?;
            let ikm_ctx = if input_key_material.is_null() {
                None
            } else {
                unsafe {
                    Some(
                        v.secret_ffi_converter
                            .secret_from_ffi(*input_key_material)?,
                    )
                }
            };

            let array: &[FfiSecretKeyAttributes] =
                unsafe { slice::from_raw_parts(derived_outputs_attributes, derived_outputs_count) };

            let output_attributes: Vec<SecretKeyAttributes> =
                array.iter().map(|x| x.into()).collect();

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
                .hkdf_sha256(&salt_ctx, b"", ikm_ctx.as_ref(), output_attributes)?
                .iter()
                .map(
                    |x| v.secret_ffi_converter.secret_into_ffi(x).unwrap(), /* FIXME */
                )
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
    let output = VAULTS.call_with_result_mut(
        &mut err,
        context,
        |v| -> Result<ByteBuffer, VaultFailError> {
            let ctx = v.secret_ffi_converter.secret_from_ffi(secret)?;
            let mut nonce_vec = vec![0; 12 - 2];
            nonce_vec.extend_from_slice(&nonce.to_be_bytes());
            let ciphertext =
                v.vault
                    .aead_aes_gcm_encrypt(&ctx, plaintext, &nonce_vec, additional_data)?;
            Ok(ByteBuffer::from_vec(ciphertext))
        },
    );
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
    let output = VAULTS.call_with_result_mut(
        &mut err,
        context,
        |v| -> Result<ByteBuffer, VaultFailError> {
            let ctx = v.secret_ffi_converter.secret_from_ffi(secret)?;
            let mut nonce_vec = vec![0; 12 - 2];
            nonce_vec.extend_from_slice(&nonce.to_be_bytes());
            let plain = v.vault.aead_aes_gcm_decrypt(
                &ctx,
                ciphertext_and_tag,
                &nonce_vec,
                additional_data,
            )?;
            Ok(ByteBuffer::from_vec(plain))
        },
    );
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
