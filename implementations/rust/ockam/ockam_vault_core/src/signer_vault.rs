use crate::secret::Secret;
use zeroize::Zeroize;

/// Trait with sign functionality
pub trait SignerVault: Zeroize {
    /// Generate a signature
    fn sign(&mut self, secret_key: &Secret, data: &[u8]) -> ockam_core::Result<[u8; 64]>;
}
