use crate::{Buffer, Secret};
use ockam_core::Result;
use zeroize::Zeroize;

/// Trait with symmetric encryption
pub trait SymmetricVault: Zeroize {
    /// Encrypt a payload using AES-GCM
    fn aead_aes_gcm_encrypt(
        &mut self,
        context: &Secret,
        plaintext: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> Result<Buffer<u8>>;
    /// Decrypt a payload using AES-GCM
    fn aead_aes_gcm_decrypt(
        &mut self,
        context: &Secret,
        cipher_text: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> Result<Buffer<u8>>;
}
