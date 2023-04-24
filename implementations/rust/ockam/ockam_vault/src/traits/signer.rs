use crate::{KeyId, Signature};
use ockam_core::{async_trait, compat::boxed::Box, Result};

/// Defines the Vault interface for Signing.
#[async_trait]
pub trait Signer {
    /// Generate a `Signature` for the given data using the given `Secret` key.
    async fn sign(&self, key_id: &KeyId, data: &[u8]) -> Result<Signature>;
}
