use crate::{Buffer, KeyId};
use ockam_core::{async_trait, compat::boxed::Box, Result};

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

#[cfg(feature = "vault_tests")]
pub mod tests {
    use crate::{EphemeralSecretsStore, SecretAttributes, SymmetricVault};

    /// This test checks that we can use an ephemeral secret to encrypt and decrypt data
    pub async fn test_encrypt_decrypt(vault: &mut (impl SymmetricVault + EphemeralSecretsStore)) {
        let message = b"Ockam Test Message";
        let nonce = b"TestingNonce";
        let aad = b"Extra payload data";
        let attributes = SecretAttributes::Aes128;

        let ctx = &vault.create_ephemeral_secret(attributes).await.unwrap();
        let res = vault
            .aead_aes_gcm_encrypt(ctx, message.as_ref(), nonce.as_ref(), aad.as_ref())
            .await;
        assert!(res.is_ok());
        let mut ciphertext = res.unwrap();
        let res = vault
            .aead_aes_gcm_decrypt(ctx, ciphertext.as_slice(), nonce.as_ref(), aad.as_ref())
            .await;
        assert!(res.is_ok());
        let plaintext = res.unwrap();
        assert_eq!(plaintext, message.to_vec());
        ciphertext[0] ^= 0xb4;
        ciphertext[1] ^= 0xdc;
        let res = vault
            .aead_aes_gcm_decrypt(ctx, ciphertext.as_slice(), nonce.as_ref(), aad.as_ref())
            .await;
        assert!(res.is_err());
    }
}
