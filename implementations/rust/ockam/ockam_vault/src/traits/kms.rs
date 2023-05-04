use crate::{PublicKey, SecretAttributes, Signature};
use ockam_core::compat::boxed::Box;
use ockam_core::{async_trait, KeyId, Result};

/// This trait provides the main functions of a KMS
///   - create and persist secrets
///   - delete secrets
///   - return the public key for a given key id
///   - return the key id for a given public key
///   - use a secret to sign a message
///   - use a public key to verify a message signature
#[async_trait]
pub trait Kms: Sync + Send {
    /// Create a new secret and return its key id
    async fn create_secret(&self, attributes: SecretAttributes) -> Result<KeyId>;

    /// Get the public key from a secret
    async fn get_public_key(&self, key_id: &KeyId) -> Result<PublicKey>;

    /// Return the key id corresponding to a given public key
    async fn get_key_id(&self, public_key: &PublicKey) -> Result<KeyId>;

    /// Return the secret attributes for a given key id
    async fn get_attributes(&self, key_id: &KeyId) -> Result<SecretAttributes>;

    /// Delete a secret
    async fn delete_secret(&self, key_id: KeyId) -> Result<bool>;

    /// Sign a message with a given key
    async fn sign(&self, key_id: &KeyId, message: &[u8]) -> Result<Signature>;

    /// Verify a message signature
    async fn verify(
        &self,
        public_key: &PublicKey,
        message: &[u8],
        signature: &Signature,
    ) -> Result<bool>;
}
