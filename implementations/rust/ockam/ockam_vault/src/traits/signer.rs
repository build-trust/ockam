use crate::{KeyId, PublicKey, Signature};
use ockam_core::{async_trait, compat::boxed::Box, Result};

/// Defines the Vault interface for Signing.
#[async_trait]
pub trait Signer: Send + Sync {
    /// Generate a `Signature` for the given data using the given `Secret` key.
    async fn sign(&self, key_id: &KeyId, data: &[u8]) -> Result<Signature>;

    /// Verify a signature for the given data using the given public key.
    async fn verify(
        &self,
        public_key: &PublicKey,
        data: &[u8],
        signature: &Signature,
    ) -> Result<bool>;
}

/// Tests for implementations of the Signer trait
#[cfg(feature = "vault_tests")]
pub mod tests {
    use super::*;
    use crate::{SecretAttributes, SecretsStore};
    use ockam_core::KeyId;

    /// This test checks that an ephemeral secret can be used to sign data and that we can verify the signature
    pub async fn test_sign_and_verify_ephemeral_secret(vault: &mut (impl Signer + SecretsStore)) {
        for attributes in [SecretAttributes::Ed25519] {
            let secret = vault.create_ephemeral_secret(attributes).await.unwrap();
            sign_and_verify(vault, &secret).await;
        }
    }

    /// This test checks that a persistent secret can be used to sign data and that we can verify the signature
    pub async fn test_sign_and_verify_persistent_secret(vault: &mut (impl Signer + SecretsStore)) {
        for attributes in [SecretAttributes::Ed25519] {
            let secret = vault.create_persistent_secret(attributes).await.unwrap();
            sign_and_verify(vault, &secret).await;
        }
    }

    /// Use a secret to sign data, then verify the signature
    async fn sign_and_verify(vault: &mut (impl Signer + SecretsStore + Sized), secret: &KeyId) {
        let res = vault.sign(secret, b"hello world!").await;
        assert!(res.is_ok());
        let pubkey = vault.get_public_key(secret).await.unwrap();
        let signature = res.unwrap();
        let res = vault
            .verify(&pubkey, b"hello world!", &signature)
            .await
            .unwrap();
        assert!(res);
    }
}
