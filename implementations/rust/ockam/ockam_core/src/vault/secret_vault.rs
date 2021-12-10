use crate::vault::{PublicKey, Secret, SecretAttributes, SecretKey};
use crate::Result;
use crate::{async_trait, compat::boxed::Box};

/// [`Secret`]-management functionality
#[async_trait]
pub trait SecretVault {
    /// Generate fresh secret with given attributes
    async fn secret_generate(&mut self, attributes: SecretAttributes) -> Result<Secret>;
    /// Import a secret with given attributes from binary form into the vault
    async fn secret_import(
        &mut self,
        secret: &[u8],
        attributes: SecretAttributes,
    ) -> Result<Secret>;
    /// Export a secret key to the binary form represented as [`SecretKey`]
    async fn secret_export(&mut self, context: &Secret) -> Result<SecretKey>;
    /// Get the attributes for a secret
    async fn secret_attributes_get(&mut self, context: &Secret) -> Result<SecretAttributes>;
    /// Return the associated public key given the secret key
    async fn secret_public_key_get(&mut self, context: &Secret) -> Result<PublicKey>;
    /// Remove a secret from the vault
    async fn secret_destroy(&mut self, context: Secret) -> Result<()>;
}
