use crate::vault_types::{FfiSecretAttributes, SecretKeyHandle};
use crate::{check_buffer, FfiError, FfiOckamError};
use crate::{FfiVaultFatPointer, FfiVaultType};
use core::slice;
use lazy_static::lazy_static;
use ockam_core::compat::sync::Arc;
use ockam_core::vault::{
    AsymmetricVault, Hasher, PublicKey, Secret, SecretAttributes, SecretVault, SymmetricVault,
};
use ockam_core::{Error, Result};
use ockam_vault::Vault;
use std::future::Future;
use tokio::runtime::Runtime;
use tokio::sync::RwLock;
use tokio::task;

/// FFI Vault trait. See documentation for individual sub-traits for details.
pub trait FfiVault: SecretVault + Hasher + SymmetricVault + AsymmetricVault + Send {}

impl<D> FfiVault for D where D: SecretVault + Hasher + SymmetricVault + AsymmetricVault + Send {}

lazy_static! {
    static ref SOFTWARE_VAULTS: RwLock<Vec<Vault>> = RwLock::new(vec![]);
    static ref RUNTIME: Arc<Runtime> = Arc::new(Runtime::new().unwrap());
}

fn get_runtime() -> Arc<Runtime> {
    RUNTIME.clone()
}

fn block_future<F>(f: F) -> <F as Future>::Output
where
    F: Future,
{
    let rt = get_runtime();
    task::block_in_place(move || {
        let local = task::LocalSet::new();
        local.block_on(&rt, f)
    })
}

async fn get_vault(context: FfiVaultFatPointer) -> Result<Vault> {
    match context.vault_type() {
        FfiVaultType::Software => {
            let item = SOFTWARE_VAULTS
                .read()
                .await
                .get(context.handle() as usize)
                .ok_or(FfiError::VaultNotFound)?
                .clone();

            Ok(item)
        }
    }
}

/// Create and return a default Ockam Vault.
#[no_mangle]
pub extern "C" fn ockam_vault_default_init(context: &mut FfiVaultFatPointer) -> FfiOckamError {
    handle_panics(|| {
        // TODO: handle logging
        let handle = block_future(async move {
            let mut write_lock = SOFTWARE_VAULTS.write().await;
            write_lock.push(Vault::default());
            write_lock.len() - 1
        });

        *context = FfiVaultFatPointer::new(handle as u64, FfiVaultType::Software);

        Ok(())
    })
}

/// Compute the SHA-256 hash on `input` and put the result in `digest`.
/// `digest` must be 32 bytes in length.
#[no_mangle]
pub extern "C" fn ockam_vault_sha256(
    context: FfiVaultFatPointer,
    input: *const u8,
    input_length: u32,
    digest: *mut u8,
) -> FfiOckamError {
    handle_panics(|| {
        check_buffer!(input);
        check_buffer!(digest);

        let input = unsafe { core::slice::from_raw_parts(input, input_length as usize) };

        let res = block_future(async move {
            let v = get_vault(context).await?;
            v.sha256(input).await
        })?;

        unsafe {
            std::ptr::copy_nonoverlapping(res.as_ptr(), digest, res.len());
        }
        Ok(())
    })
}

/// Generate a secret key with the specific attributes.
/// Returns a handle for the secret.
#[no_mangle]
pub extern "C" fn ockam_vault_secret_generate(
    context: FfiVaultFatPointer,
    secret: &mut SecretKeyHandle,
    attributes: FfiSecretAttributes,
) -> FfiOckamError {
    handle_panics(|| {
        *secret = block_future(async move {
            let v = get_vault(context).await?;
            let atts = attributes.try_into()?;
            let ctx = v.secret_generate(atts).await?;
            Ok::<u64, Error>(ctx.index() as u64)
        })?;
        Ok(())
    })
}

/// Import a secret key with the specific handle and attributes.
#[no_mangle]
pub extern "C" fn ockam_vault_secret_import(
    context: FfiVaultFatPointer,
    secret: &mut SecretKeyHandle,
    attributes: FfiSecretAttributes,
    input: *mut u8,
    input_length: u32,
) -> FfiOckamError {
    handle_panics(|| {
        check_buffer!(input, input_length);
        *secret = block_future(async move {
            let v = get_vault(context).await?;
            let atts = attributes.try_into()?;

            let secret_data = unsafe { core::slice::from_raw_parts(input, input_length as usize) };

            let ctx = v.secret_import(secret_data, atts).await?;
            Ok::<u64, Error>(ctx.index() as u64)
        })?;
        Ok(())
    })
}

/// Export a secret key with the specific handle to the `output_buffer`.
#[no_mangle]
pub extern "C" fn ockam_vault_secret_export(
    context: FfiVaultFatPointer,
    secret: SecretKeyHandle,
    output_buffer: *mut u8,
    output_buffer_size: u32,
    output_buffer_length: &mut u32,
) -> FfiOckamError {
    *output_buffer_length = 0;
    handle_panics(|| {
        block_future(async move {
            let v = get_vault(context).await?;
            let ctx = Secret::new(secret as usize);
            let key = v.secret_export(&ctx).await?;
            if output_buffer_size < key.as_ref().len() as u32 {
                return Err(FfiError::BufferTooSmall.into());
            }
            *output_buffer_length = key.as_ref().len() as u32;

            unsafe {
                std::ptr::copy_nonoverlapping(
                    key.as_ref().as_ptr(),
                    output_buffer,
                    key.as_ref().len(),
                );
            };
            Ok::<(), Error>(())
        })?;
        Ok(())
    })
}

/// Get the public key, given a secret key, and copy it to the output buffer.
#[no_mangle]
pub extern "C" fn ockam_vault_secret_publickey_get(
    context: FfiVaultFatPointer,
    secret: SecretKeyHandle,
    output_buffer: *mut u8,
    output_buffer_size: u32,
    output_buffer_length: &mut u32,
) -> FfiOckamError {
    *output_buffer_length = 0;
    handle_panics(|| {
        block_future(async move {
            let v = get_vault(context).await?;
            let ctx = Secret::new(secret as usize);
            let key = v.secret_public_key_get(&ctx).await?;
            if output_buffer_size < key.as_ref().len() as u32 {
                return Err(FfiError::BufferTooSmall.into());
            }
            *output_buffer_length = key.as_ref().len() as u32;

            unsafe {
                std::ptr::copy_nonoverlapping(
                    key.as_ref().as_ptr(),
                    output_buffer,
                    key.as_ref().len(),
                );
            };
            Ok::<(), Error>(())
        })?;
        Ok(())
    })
}

/// Retrieve the attributes for a specified secret.
#[no_mangle]
pub extern "C" fn ockam_vault_secret_attributes_get(
    context: FfiVaultFatPointer,
    secret: SecretKeyHandle,
    attributes: &mut FfiSecretAttributes,
) -> FfiOckamError {
    handle_panics(|| {
        *attributes = block_future(async move {
            let v = get_vault(context).await?;
            let ctx = Secret::new(secret as usize);
            let atts = v.secret_attributes_get(&ctx).await?;
            Ok::<FfiSecretAttributes, Error>(atts.into())
        })?;
        Ok(())
    })
}

/// Delete an ockam vault secret.
#[no_mangle]
pub extern "C" fn ockam_vault_secret_destroy(
    context: FfiVaultFatPointer,
    secret: SecretKeyHandle,
) -> FfiOckamError {
    match block_future(async move {
        let v = get_vault(context).await?;
        let ctx = Secret::new(secret as usize);
        v.secret_destroy(ctx).await?;
        Ok::<(), Error>(())
    }) {
        Ok(_) => FfiOckamError::none(),
        Err(err) => err.into(),
    }
}

/// Perform an ECDH operation on the supplied Ockam Vault `secret` and `peer_publickey`. The result
/// is an Ockam Vault secret of unknown type.
#[no_mangle]
pub extern "C" fn ockam_vault_ecdh(
    context: FfiVaultFatPointer,
    secret: SecretKeyHandle,
    peer_publickey: *const u8,
    peer_publickey_length: u32,
    shared_secret: &mut SecretKeyHandle,
) -> FfiOckamError {
    handle_panics(|| {
        check_buffer!(peer_publickey, peer_publickey_length);

        let peer_publickey =
            unsafe { core::slice::from_raw_parts(peer_publickey, peer_publickey_length as usize) };

        *shared_secret = block_future(async move {
            let v = get_vault(context).await?;
            let ctx = Secret::new(secret as usize);
            let atts = v.secret_attributes_get(&ctx).await?;
            let pubkey = PublicKey::new(peer_publickey.to_vec(), atts.stype());
            let shared_ctx = v.ec_diffie_hellman(&ctx, &pubkey).await?;
            Ok::<u64, Error>(shared_ctx.index() as u64)
        })?;
        Ok(())
    })
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
    handle_panics(|| {
        let derived_outputs_count = derived_outputs_count as usize;

        block_future(async move {
            let v = get_vault(context).await?;
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
            let hkdf_output = v
                .hkdf_sha256(&salt_ctx, b"", ikm_ctx, output_attributes)
                .await?;

            let hkdf_output: Vec<SecretKeyHandle> =
                hkdf_output.into_iter().map(|x| x.index() as u64).collect();

            unsafe {
                std::ptr::copy_nonoverlapping(
                    hkdf_output.as_ptr(),
                    derived_outputs,
                    derived_outputs_count,
                )
            };
            Ok::<(), Error>(())
        })?;
        Ok(())
    })
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
    *ciphertext_and_tag_length = 0;
    handle_panics(|| {
        check_buffer!(additional_data);
        check_buffer!(plaintext);

        let additional_data = unsafe {
            core::slice::from_raw_parts(additional_data, additional_data_length as usize)
        };

        let plaintext =
            unsafe { core::slice::from_raw_parts(plaintext, plaintext_length as usize) };

        block_future(async move {
            let v = get_vault(context).await?;
            let ctx = Secret::new(secret as usize);
            let mut nonce_vec = vec![0; 12 - 2];
            nonce_vec.extend_from_slice(&nonce.to_be_bytes());
            let ciphertext = v
                .aead_aes_gcm_encrypt(&ctx, plaintext, &nonce_vec, additional_data)
                .await?;

            if ciphertext_and_tag_size < ciphertext.len() as u32 {
                return Err(FfiError::BufferTooSmall.into());
            }
            *ciphertext_and_tag_length = ciphertext.len() as u32;

            unsafe {
                std::ptr::copy_nonoverlapping(
                    ciphertext.as_ptr(),
                    ciphertext_and_tag,
                    ciphertext.len(),
                )
            };
            Ok::<(), Error>(())
        })?;
        Ok(())
    })
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
    *plaintext_length = 0;
    handle_panics(|| {
        check_buffer!(ciphertext_and_tag, ciphertext_and_tag_length);
        check_buffer!(additional_data);

        let additional_data = unsafe {
            core::slice::from_raw_parts(additional_data, additional_data_length as usize)
        };

        let ciphertext_and_tag = unsafe {
            core::slice::from_raw_parts(ciphertext_and_tag, ciphertext_and_tag_length as usize)
        };

        block_future(async move {
            let v = get_vault(context).await?;
            let ctx = Secret::new(secret as usize);
            let mut nonce_vec = vec![0; 12 - 2];
            nonce_vec.extend_from_slice(&nonce.to_be_bytes());
            let plain = v
                .aead_aes_gcm_decrypt(&ctx, ciphertext_and_tag, &nonce_vec, additional_data)
                .await?;
            if plaintext_size < plain.len() as u32 {
                return Err(FfiError::BufferTooSmall.into());
            }
            *plaintext_length = plain.len() as u32;

            unsafe { std::ptr::copy_nonoverlapping(plain.as_ptr(), plaintext, plain.len()) };
            Ok::<(), Error>(())
        })?;
        Ok(())
    })
}

/// De-initialize an Ockam Vault.
#[no_mangle]
pub extern "C" fn ockam_vault_deinit(context: FfiVaultFatPointer) -> FfiOckamError {
    handle_panics(|| {
        block_future(async move {
            match context.vault_type() {
                FfiVaultType::Software => {
                    let handle = context.handle() as usize;
                    let mut v = SOFTWARE_VAULTS.write().await;
                    if handle < v.len() {
                        v.remove(handle);
                        Ok(())
                    } else {
                        Err(FfiError::VaultNotFound)
                    }
                }
            }
        })?;
        Ok(())
    })
}

fn handle_panics<F>(f: F) -> FfiOckamError
where
    F: FnOnce() -> Result<(), FfiOckamError>,
{
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
    match result {
        // No error.
        Ok(Ok(())) => FfiOckamError::none(),
        // Failed with a specific ockam error:
        Ok(Err(e)) => e,
        // Panicked
        Err(e) => {
            // Force an abort if either:
            //
            // - `e` panics during its `Drop` impl.
            // - `FfiOckamError::from(FfiError)` panics.
            //
            // Both of these are extremely unlikely, but possible.
            let panic_guard = AbortOnDrop;
            drop(e);
            let ret = FfiOckamError::from(FfiError::UnexpectedPanic);
            core::mem::forget(panic_guard);
            ret
        }
    }
}

/// Aborts on drop, used to guard against panics in a section of code.
///
/// Correct usage should `mem::forget` this struct after the non-panicking
/// section.
struct AbortOnDrop;
impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        eprintln!("Panic from error drop, aborting!");
        std::process::abort();
    }
}
