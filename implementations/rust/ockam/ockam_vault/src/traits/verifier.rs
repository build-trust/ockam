use crate::{PublicKey, Signature};
use ockam_core::{async_trait, compat::boxed::Box, Result};

/// Defines the Vault interface for `Signature` verification.
#[async_trait]
pub trait Verifier {
    /// Verify a signature for the given data using the given public key.
    async fn verify(
        &self,
        signature: &Signature,
        public_key: &PublicKey,
        data: &[u8],
    ) -> Result<bool>;
}

#[async_trait]
impl<V: Verifier + Send + Sync + 'static> Verifier for &V {
    async fn verify(&self, s: &Signature, k: &PublicKey, d: &[u8]) -> Result<bool> {
        V::verify(self, s, k, d).await
    }
}
