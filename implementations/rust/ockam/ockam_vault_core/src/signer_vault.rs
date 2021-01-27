use crate::secret::Secret;
use ockam_core::Error;
use zeroize::Zeroize;

/// Trait with sign functionality
pub trait SignerVault: Zeroize {
    /// Generate a signature
    fn sign(&mut self, secret_key: &Secret, data: &[u8]) -> Result<[u8; 64], Error>;
}
