use crate::secret::Secret;
use crate::types::{PublicKey, SecretAttributes, SecretKey};
use ockam_core::Result;
use ockam_core::{async_trait, compat::boxed::Box};

/// [`Secret`]-management functionality
#[async_trait]
pub trait SecretVault: Send + Sync {
    /// Generate fresh secret with given attributes
    async fn secret_generate(&self, attributes: SecretAttributes) -> Result<Secret>;
    /// Import a secret with given attributes from binary form into the vault
    async fn secret_import(
        &self,
        secret: &[u8],
        attributes: SecretAttributes,
    ) -> Result<Secret>;
    /// Export a secret key to the binary form represented as [`SecretKey`]
    async fn secret_export(&self, context: &Secret) -> Result<SecretKey>;
    /// Get the attributes for a secret
    async fn secret_attributes_get(&self, context: &Secret) -> Result<SecretAttributes>;
    /// Return the associated public key given the secret key
    async fn secret_public_key_get(&self, context: &Secret) -> Result<PublicKey>;
    /// Remove a secret from the vault
    async fn secret_destroy(&self, context: Secret) -> Result<()>;
}

#[async_trait]
impl<V: ?Sized + SecretVault + Send + Sync> SecretVault for ockam_core::compat::sync::Arc<V> {
    async fn secret_generate(&self, attributes: SecretAttributes) -> Result<Secret> {
        V::secret_generate(&**self, attributes).await
    }
    async fn secret_import(&self, secret: &[u8], attributes: SecretAttributes) -> Result<Secret> {
        V::secret_import(&**self, secret, attributes).await
    }
    async fn secret_export(&self, context: &Secret) -> Result<SecretKey> {
        V::secret_export(&**self, context).await
    }
    async fn secret_attributes_get(&self, context: &Secret) -> Result<SecretAttributes> {
        V::secret_attributes_get(&**self, context).await
    }
    async fn secret_public_key_get(&self, context: &Secret) -> Result<PublicKey> {
        V::secret_public_key_get(&**self, context).await
    }
    async fn secret_destroy(&self, context: Secret) -> Result<()> {
        V::secret_destroy(&**self, context).await
    }
}
