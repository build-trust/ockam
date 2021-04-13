use crate::secret::Secret;
use crate::types::{PublicKey, SecretAttributes, SecretKey};
use ockam_core::Result;
use zeroize::Zeroize;

/// [`Secret`]-management functionality
pub trait SecretVault: Zeroize {
    /// Generate fresh secret with given attributes
    fn secret_generate(&mut self, attributes: SecretAttributes) -> Result<Secret>;
    /// Import a secret with given attributes from binary form into the vault
    fn secret_import(&mut self, secret: &[u8], attributes: SecretAttributes) -> Result<Secret>;
    /// Export a secret key to the binary form represented as [`SecretKey`]
    fn secret_export(&mut self, context: &Secret) -> Result<SecretKey>;
    /// Get the attributes for a secret
    fn secret_attributes_get(&mut self, context: &Secret) -> Result<SecretAttributes>;
    /// Return the associated public key given the secret key
    fn secret_public_key_get(&mut self, context: &Secret) -> Result<PublicKey>;
    /// Remove a secret from the vault
    fn secret_destroy(&mut self, context: Secret) -> Result<()>;
}
