use crate::vault::{Secret, Signature};
use crate::Result;
use crate::{async_trait, compat::boxed::Box};

/// Defines the Vault interface for Signing.
#[async_trait]
pub trait Signer {
    /// Generate a `Signature` for the given data using the given `Secret` key.
    async fn sign(&self, secret_key: &Secret, data: &[u8]) -> Result<Signature>;
}
