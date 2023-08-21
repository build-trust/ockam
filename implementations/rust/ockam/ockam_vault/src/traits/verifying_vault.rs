use crate::{PublicKey, Signature};
use ockam_core::{async_trait, compat::boxed::Box, Result};

/// Vault used for verification (signature verification, sha256)
#[async_trait]
pub trait VerifyingVault: Send + Sync + 'static {
    /// Compute sha256
    async fn sha256(&self, data: &[u8]) -> Result<[u8; 32]>;

    /// Verify a signature
    async fn verify(
        &self,
        public_key: &PublicKey,
        data: &[u8],
        signature: &Signature,
    ) -> Result<bool>;
}
