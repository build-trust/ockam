use crate::secret::Secret;
use crate::Signature;
use ockam_core::Result;
use ockam_core::{async_trait, compat::boxed::Box};

/// Signing functionality
#[async_trait]
pub trait Signer: Send + Sync {
    /// Generate a signature for given data using given secret key
    async fn sign(&self, secret_key: &Secret, data: &[u8]) -> Result<Signature>;
}

#[async_trait]
impl<V: ?Sized + Signer> Signer for ockam_core::compat::sync::Arc<V> {
    async fn sign(&self, secret_key: &Secret, data: &[u8]) -> Result<Signature> {
        V::sign(&**self, secret_key, data).await
    }
}
