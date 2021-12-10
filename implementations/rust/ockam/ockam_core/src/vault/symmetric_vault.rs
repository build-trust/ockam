use crate::vault::{Buffer, Secret};
use crate::Result;
use crate::{async_trait, compat::boxed::Box};

/// Trait with symmetric encryption
#[async_trait]
pub trait SymmetricVault {
    /// Encrypt a payload using AES-GCM
    async fn aead_aes_gcm_encrypt(
        &mut self,
        context: &Secret,
        plaintext: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> Result<Buffer<u8>>;
    /// Decrypt a payload using AES-GCM
    async fn aead_aes_gcm_decrypt(
        &mut self,
        context: &Secret,
        cipher_text: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> Result<Buffer<u8>>;
}
