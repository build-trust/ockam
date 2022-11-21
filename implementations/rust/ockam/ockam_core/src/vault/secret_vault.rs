use crate::vault::{KeyId, PublicKey, SecretAttributes};
use crate::Result;
use crate::{async_trait, compat::boxed::Box};

use super::Secret;

/// Defines the `Secret` management interface for Ockam Vaults.
///
/// # Examples
///
/// See `ockam_vault::SoftwareVault` for a usage example.
///
#[async_trait]
pub trait SecretVault {
    /// Generate a fresh secret with the given attributes.
    async fn secret_generate(&self, attributes: SecretAttributes) -> Result<KeyId>;
    /// Import a secret with the given attributes from binary form into the vault.
    async fn secret_import(&self, secret: Secret, attributes: SecretAttributes) -> Result<KeyId>;
    /// Export a secret key to the binary form represented as [`SecretKey`].
    async fn secret_export(&self, key_id: &KeyId) -> Result<Secret>;
    /// Return the attributes for a secret.
    async fn secret_attributes_get(&self, key_id: &KeyId) -> Result<SecretAttributes>;
    /// Return the associated public key given the secret key.
    async fn secret_public_key_get(&self, key_id: &KeyId) -> Result<PublicKey>;
    /// Remove a secret from the vault.
    async fn secret_destroy(&self, key_id: KeyId) -> Result<()>;
}
