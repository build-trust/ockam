use crate::vault::{Secret, Signature};
use crate::{async_trait, compat::boxed::Box, Result};

/// Signing functionality
#[async_trait]
pub trait Signer {
    /// Generate a signature  for given data using given secret key
    async fn sign(&mut self, secret_key: &Secret, data: &[u8]) -> Result<Signature>;
}
