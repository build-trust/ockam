use crate::{PublicKey, Signature};
use ockam_core::Result;
use ockam_core::{async_trait, compat::boxed::Box};

/// Signature verification functionality
#[async_trait]
pub trait Verifier: Send + Sync {
    /// Verify a signature for given data using given public key
    async fn verify(
        &self,
        signature: &Signature,
        public_key: &PublicKey,
        data: &[u8],
    ) -> Result<bool>;
}

#[async_trait]
impl<V: ?Sized + Verifier> Verifier for ockam_core::compat::sync::Arc<V> {
    async fn verify(
        &self,
        signature: &Signature,
        public_key: &PublicKey,
        data: &[u8],
    ) -> Result<bool> {
        V::verify(&**self, signature, public_key, data).await
    }
}
