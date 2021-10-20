use crate::{PublicKey, Signature};
use ockam_core::Result;
use ockam_core::{async_trait, compat::boxed::Box};

/// Signature verification functionality
#[async_trait]
pub trait Verifier {
    /// Verify a signature for given data using given public key
    async fn verify(
        &mut self,
        signature: &Signature,
        public_key: &PublicKey,
        data: &[u8],
    ) -> Result<bool>;
}
