use crate::{AeadSecretKeyHandle, SecretBuffer};
use alloc::boxed::Box;
use ockam_core::{async_trait, Result};

/// Vault for verifying signatures and computing SHA-256.
#[async_trait]
pub trait VaultForEncryptionAtRest: Send + Sync + 'static {
    /// Perform AEAD encryption.
    /// Nonce is generated internally.
    /// The plain_text[0..AES_NONCE_LENGTH] will be overwritten by the nonce, use
    /// plain_text[AES_NONCE_LENGTH..] to pass the actual data to encrypt.
    async fn aead_encrypt(
        &self,
        secret_key_handle: &AeadSecretKeyHandle,
        plain_text: &mut [u8],
        aad: &[u8],
    ) -> Result<()>;

    /// Perform AEAD decryption.
    /// The nonce is the first AES_NONCE_LENGTH bytes of the cipher_text.
    /// The cipher_text will be decrypted in place.
    async fn aead_decrypt<'a>(
        &self,
        secret_key_handle: &AeadSecretKeyHandle,
        rekey_counter: u16,
        cipher_text: &'a mut [u8],
        aad: &[u8],
    ) -> Result<&'a mut [u8]>;

    /// Rekey, delete the old one and return the new key handle
    async fn rekey_and_delete(
        &self,
        secret_key_handle: &AeadSecretKeyHandle,
    ) -> Result<AeadSecretKeyHandle>;

    /// Import an AES-GCM key and return a handle to it
    async fn import_aead_key(&self, secret: SecretBuffer) -> Result<AeadSecretKeyHandle>;
}
