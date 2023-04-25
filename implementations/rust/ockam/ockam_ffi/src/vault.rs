use crate::vault_types::{FfiSecretAttributes, SecretKeyHandle};
use crate::{check_buffer, FfiError, FfiOckamError};
use crate::{FfiVaultFatPointer, FfiVaultType};
use core::{future::Future, result::Result as StdResult, slice};
use futures::future::join_all;
use lazy_static::lazy_static;
use ockam_core::compat::collections::BTreeMap;
use ockam_core::compat::sync::Arc;
use ockam_core::vault::{
    AsymmetricVault, Hasher, KeyId, PublicKey, Secret, SecretAttributes, SecretKey, SecretVault,
    SymmetricVault,
};
use ockam_core::{Error, Result};
use ockam_vault::Vault;
use tokio::{runtime::Runtime, sync::RwLock, task};

#[derive(Default)]
struct SecretsMapping {
    mapping: BTreeMap<u64, KeyId>,
    last_index: u64,
}

impl SecretsMapping {
    fn insert(&mut self, key_id: KeyId) -> u64 {
        self.last_index += 1;

        self.mapping.insert(self.last_index, key_id);

        self.last_index
    }

    fn get(&self, index: u64) -> Result<KeyId> {
        Ok(self
            .mapping
            .get(&index)
            .cloned()
            .ok_or(FfiError::EntryNotFound)?)
    }

    fn take(&mut self, index: u64) -> Result<KeyId> {
        Ok(self.mapping.remove(&index).ok_or(FfiError::EntryNotFound)?)
    }
}

#[derive(Clone, Default)]
struct VaultEntry {
    vault: Vault,
    secrets_mapping: Arc<RwLock<SecretsMapping>>,
}

impl VaultEntry {
    async fn insert(&self, key_id: KeyId) -> u64 {
        self.secrets_mapping.write().await.insert(key_id)
    }

    async fn get(&self, index: u64) -> Result<KeyId> {
        self.secrets_mapping.read().await.get(index)
    }

    async fn take(&self, index: u64) -> Result<KeyId> {
        self.secrets_mapping.write().await.take(index)
    }
}

lazy_static! {
    static ref SOFTWARE_VAULTS: RwLock<Vec<VaultEntry>> = RwLock::new(vec![]);
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

async fn get_vault_entry(context: FfiVaultFatPointer) -> Result<VaultEntry> {
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
            write_lock.push(Default::default());
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
            let entry = get_vault_entry(context).await?;
            entry.vault.sha256(input).await
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
            let entry = get_vault_entry(context).await?;
            let atts = attributes.try_into()?;
            let key_id = entry.vault.secret_generate(atts).await?;

            let index = entry.insert(key_id).await;

            Ok::<u64, Error>(index)
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
            let entry = get_vault_entry(context).await?;
            let atts = attributes.try_into()?;

            let secret_data = unsafe { core::slice::from_raw_parts(input, input_length as usize) };

            let secret = Secret::Key(SecretKey::new(secret_data.to_vec()));
            let key_id = entry.vault.secret_import(secret, atts).await?;

            let index = entry.insert(key_id).await;

            Ok::<u64, Error>(index)
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
            let entry = get_vault_entry(context).await?;
            let key_id = entry.get(secret).await?;
            let key = entry.vault.secret_export(&key_id).await?;
            if output_buffer_size < key.try_as_key()?.as_ref().len() as u32 {
                return Err(FfiError::BufferTooSmall.into());
            }
            *output_buffer_length = key.try_as_key()?.as_ref().len() as u32;

            unsafe {
                std::ptr::copy_nonoverlapping(
                    key.try_as_key()?.as_ref().as_ptr(),
                    output_buffer,
                    key.try_as_key()?.as_ref().len(),
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
            let entry = get_vault_entry(context).await?;
            let key_id = entry.get(secret).await?;
            let key = entry.vault.secret_public_key_get(&key_id).await?;
            if output_buffer_size < key.data().len() as u32 {
                return Err(FfiError::BufferTooSmall.into());
            }
            *output_buffer_length = key.data().len() as u32;

            unsafe {
                std::ptr::copy_nonoverlapping(key.data().as_ptr(), output_buffer, key.data().len());
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
            let entry = get_vault_entry(context).await?;
            let key_id = entry.get(secret).await?;
            let atts = entry.vault.secret_attributes_get(&key_id).await?;
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
        let entry = get_vault_entry(context).await?;
        let key_id = entry.take(secret).await?;
        entry.vault.secret_destroy(key_id).await?;
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
            let entry = get_vault_entry(context).await?;
            let key_id = entry.get(secret).await?;
            let atts = entry.vault.secret_attributes_get(&key_id).await?;
            let pubkey = PublicKey::new(peer_publickey.to_vec(), atts.stype());
            let shared_ctx = entry.vault.ec_diffie_hellman(&key_id, &pubkey).await?;
            let index = entry.insert(shared_ctx).await;
            Ok::<u64, Error>(index)
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
            let entry = get_vault_entry(context).await?;
            let salt_key_id = entry.get(salt).await?;
            let ikm_key_id = if input_key_material.is_null() {
                None
            } else {
                let ctx = unsafe { entry.get(*input_key_material).await? };
                Some(ctx)
            };
            let ikm_key_id = ikm_key_id.as_ref();

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
            let hkdf_output = entry
                .vault
                .hkdf_sha256(&salt_key_id, b"", ikm_key_id, output_attributes)
                .await?;

            let hkdf_output: Vec<SecretKeyHandle> =
                join_all(hkdf_output.into_iter().map(|x| entry.insert(x))).await;

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
    nonce: u64,
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
            let entry = get_vault_entry(context).await?;
            let key_id = entry.get(secret).await?;
            let mut nonce_vec = vec![0; 12 - 8];
            nonce_vec.extend_from_slice(&nonce.to_be_bytes());
            let ciphertext = entry
                .vault
                .aead_aes_gcm_encrypt(&key_id, plaintext, &nonce_vec, additional_data)
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
    nonce: u64,
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
            let entry = get_vault_entry(context).await?;
            let key_id = entry.get(secret).await?;
            let mut nonce_vec = vec![0; 12 - 8];
            nonce_vec.extend_from_slice(&nonce.to_be_bytes());
            let plain = entry
                .vault
                .aead_aes_gcm_decrypt(&key_id, ciphertext_and_tag, &nonce_vec, additional_data)
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
    F: FnOnce() -> StdResult<(), FfiOckamError>,
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
