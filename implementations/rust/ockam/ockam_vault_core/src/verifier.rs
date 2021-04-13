use crate::PublicKey;
use ockam_core::Result;
use zeroize::Zeroize;

/// Signature verification functionality
pub trait Verifier: Zeroize {
    /// Verify a signature for given data using given public key
    fn verify(&mut self, signature: &[u8; 64], public_key: &PublicKey, data: &[u8])
        -> Result<bool>;
}
