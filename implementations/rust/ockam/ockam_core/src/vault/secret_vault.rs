use crate::vault::{KeyAttributes, KeyId, PublicKey};
use crate::Result;
use crate::{async_trait, compat::boxed::Box};

use super::Key;

/// Defines the `Secret` management interface for Ockam Vaults.
///
/// # Examples
///
/// See `ockam_vault::Vault` for a usage example.
///
#[async_trait]
pub trait KeyVault {
    /// Generate a fresh key with the given attributes.
    async fn generate_key(&self, attributes: KeyAttributes) -> Result<KeyId>;
    /// Import a key with the given attributes from binary form into the vault.
    async fn import_key(&self, secret: Key, attributes: KeyAttributes) -> Result<KeyId>;
    /// Export a key to the binary form represented as [`SecretKey`].
    async fn export_key(&self, key_id: &KeyId) -> Result<Key>;
    /// Return the attributes for a key.
    async fn get_key_attributes(&self, key_id: &KeyId) -> Result<KeyAttributes>;
    /// Return the associated public key given the secret key.
    async fn get_public_key(&self, key_id: &KeyId) -> Result<PublicKey>;
    /// Remove a key from the vault.
    async fn destroy_key(&self, key_id: KeyId) -> Result<()>;
}
