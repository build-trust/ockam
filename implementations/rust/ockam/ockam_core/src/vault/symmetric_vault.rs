use crate::vault::{Buffer, KeyId};
use crate::Result;
use crate::{async_trait, compat::boxed::Box};

/// Defines the Vault interface for symmetric encryption.
#[async_trait]
pub trait SymmetricVault: Send + Sync {
    /// Encrypt a payload using AES-GCM.
    async fn aead_aes_gcm_encrypt(
        &self,
        key_id: &KeyId,
        plaintext: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> Result<Buffer<u8>>;

    /// Decrypt a payload using AES-GCM.
    async fn aead_aes_gcm_decrypt(
        &self,
        key_id: &KeyId,
        cipher_text: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> Result<Buffer<u8>>;
}
