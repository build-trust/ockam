use crate::{Buffer, Secret};
use ockam_core::Result;
use ockam_core::{async_trait, compat::boxed::Box};

/// Trait with symmetric encryption
#[async_trait]
pub trait SymmetricVault: Send + Sync {
    /// Encrypt a payload using AES-GCM
    async fn aead_aes_gcm_encrypt(
        &self,
        context: &Secret,
        plaintext: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> Result<Buffer<u8>>;
    /// Decrypt a payload using AES-GCM
    async fn aead_aes_gcm_decrypt(
        &self,
        context: &Secret,
        cipher_text: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> Result<Buffer<u8>>;
}

#[async_trait]
impl<V: ?Sized + SymmetricVault> SymmetricVault for ockam_core::compat::sync::Arc<V> {
    async fn aead_aes_gcm_encrypt(
        &self,
        context: &Secret,
        plaintext: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> Result<Buffer<u8>> {
        V::aead_aes_gcm_encrypt(&**self, context, plaintext, nonce, aad).await
    }
    async fn aead_aes_gcm_decrypt(
        &self,
        context: &Secret,
        cipher_text: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> Result<Buffer<u8>> {
        V::aead_aes_gcm_decrypt(&**self, context, cipher_text, nonce, aad).await
    }
}
