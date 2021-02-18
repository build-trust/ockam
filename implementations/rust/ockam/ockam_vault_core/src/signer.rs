use crate::secret::Secret;
use zeroize::Zeroize;

/// Signing functionality
pub trait Signer: Zeroize {
    /// Generate a signature  for given data using given secret key
    fn sign(&mut self, secret_key: &Secret, data: &[u8]) -> ockam_core::Result<[u8; 64]>;
}
