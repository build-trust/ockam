use crate::secret::Secret;
use crate::Signature;
use ockam_core::compat::boxed::Box;
use ockam_core::Result;
use zeroize::Zeroize;

use ockam_core::async_trait::async_trait;
#[async_trait]
/// Signing functionality
pub trait Signer: Zeroize {
    /// Generate a signature  for given data using given secret key
    fn sign(&mut self, secret_key: &Secret, data: &[u8]) -> Result<Signature>;
    /// Generate a signature  for given data using given secret key
    async fn async_sign(&mut self, secret_key: &Secret, data: &[u8]) -> Result<Signature>;
}
