use crate::{KeyId, PublicKey, SecretAttributes, Signature};
use ockam_core::{async_trait, compat::boxed::Box, Result};

/// Vault used for Signing
#[async_trait]
pub trait SigningVault: Send + Sync + 'static {
    /// Generate a fresh random key
    async fn generate_key(&self, attributes: SecretAttributes) -> Result<KeyId>;

    /// Delete a key
    async fn delete_key(&self, key_id: KeyId) -> Result<bool>;

    /// Get corresponding [`PublicKey`]
    async fn get_public_key(&self, key_id: &KeyId) -> Result<PublicKey>;

    /// Return the [`KeyId`] given a [`PublicKey`]
    async fn get_key_id(&self, public_key: &PublicKey) -> Result<KeyId>;

    /// Sign data
    async fn sign(&self, key_id: &KeyId, data: &[u8]) -> Result<Signature>;

    /// Return the total number of all keys
    async fn number_of_keys(&self) -> Result<usize>;
}
