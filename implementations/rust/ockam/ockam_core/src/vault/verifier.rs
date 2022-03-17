use crate::vault::{PublicKey, Signature};
use crate::Result;
use crate::{async_trait, compat::boxed::Box};

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
