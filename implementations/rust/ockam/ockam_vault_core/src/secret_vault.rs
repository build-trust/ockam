use crate::secret::Secret;
use crate::types::{PublicKey, SecretAttributes, SecretKey};
use zeroize::Zeroize;

/// Vault trait with secret management functionality
pub trait SecretVault: Zeroize {
    /// Create a new secret key
    fn secret_generate(&mut self, attributes: SecretAttributes) -> ockam_core::Result<Secret>;
    /// Import a secret key into the vault
    fn secret_import(
        &mut self,
        secret: &[u8],
        attributes: SecretAttributes,
    ) -> ockam_core::Result<Secret>;
    /// Export a secret key from the vault
    fn secret_export(&mut self, context: &Secret) -> ockam_core::Result<SecretKey>;
    /// Get the attributes for a secret key
    fn secret_attributes_get(&mut self, context: &Secret) -> ockam_core::Result<SecretAttributes>;
    /// Return the associated public key given the secret key
    fn secret_public_key_get(&mut self, context: &Secret) -> ockam_core::Result<PublicKey>;
    /// Remove a secret key from the vault
    fn secret_destroy(&mut self, context: Secret) -> ockam_core::Result<()>;
}
