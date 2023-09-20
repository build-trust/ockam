use crate::{Sha256Output, Signature, VerifyingPublicKey};

use ockam_core::{async_trait, compat::boxed::Box, Result};

/// Vault for verifying signatures and computing SHA-256.
#[async_trait]
pub trait VaultForVerifyingSignatures: Send + Sync + 'static {
    ///  Compute SHA-256.
    async fn sha256(&self, data: &[u8]) -> Result<Sha256Output>;

    /// Verify a Signature.
    async fn verify_signature(
        &self,
        verifying_public_key: &VerifyingPublicKey,
        data: &[u8],
        signature: &Signature,
    ) -> Result<bool>;
}
