use crate::{PublicKey, Signature};
use ockam_core::async_trait::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_core::Result;
use zeroize::Zeroize;

#[async_trait]
/// Signature verification functionality
pub trait Verifier: Zeroize {
    /// Verify a signature for given data using given public key
    fn verify(
        &mut self,
        signature: &Signature,
        public_key: &PublicKey,
        data: &[u8],
    ) -> Result<bool>;

    /// Verify a signature for given data using given public key
    async fn async_verify(
        &mut self,
        signature: &Signature,
        public_key: &PublicKey,
        data: &[u8],
    ) -> Result<bool>;
}
