use crate::vault::{PublicKey, Secret, SecretAttributes, SecretKey};
use crate::Result;
use crate::{async_trait, compat::boxed::Box};

/// Defines the `Secret` management interface for Ockam Vaults.
///
/// # Examples
///
/// See `ockam_vault::SoftwareVault` for a usage example.
///
#[async_trait]
pub trait SecretVault {
    /// Generate a fresh secret with the given attributes.
    async fn secret_generate(&self, attributes: SecretAttributes) -> Result<Secret>;
    /// Import a secret with the given attributes from binary form into the vault.
    async fn secret_import(&self, secret: &[u8], attributes: SecretAttributes) -> Result<Secret>;
    /// Export a secret key to the binary form represented as [`SecretKey`].
    async fn secret_export(&self, context: &Secret) -> Result<SecretKey>;
    /// Return the attributes for a secret.
    async fn secret_attributes_get(&self, context: &Secret) -> Result<SecretAttributes>;
    /// Return the associated public key given the secret key.
    async fn secret_public_key_get(&self, context: &Secret) -> Result<PublicKey>;
    /// Remove a secret from the vault.
    async fn secret_destroy(&self, context: Secret) -> Result<()>;
}
