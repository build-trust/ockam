use crate::{PublicKey, Secret};
use ockam_core::Result;
use ockam_core::{async_trait, compat::{boxed::Box, sync::Arc}};

/// Vault with asymmetric encryption functionality
#[async_trait]
pub trait AsymmetricVault: Send + Sync {
    /// Compute Elliptic-Curve Diffie-Hellman using this secret key
    /// and the specified uncompressed public key
    async fn ec_diffie_hellman(
        &self,
        context: &Secret,
        peer_public_key: &PublicKey,
    ) -> Result<Secret>;
}

#[async_trait]
impl<V: ?Sized + AsymmetricVault> AsymmetricVault for Arc<V> {
    /// Compute Elliptic-Curve Diffie-Hellman using this secret key
    /// and the specified uncompressed public key
    async fn ec_diffie_hellman(
        &self,
        context: &Secret,
        peer_public_key: &PublicKey,
    ) -> Result<Secret> {
        V::ec_diffie_hellman(&**self, context, peer_public_key).await
    }
}
