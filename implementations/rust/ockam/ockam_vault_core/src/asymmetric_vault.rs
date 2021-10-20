use crate::{PublicKey, Secret};
use ockam_core::Result;
use ockam_core::{async_trait, compat::boxed::Box};

/// Vault with asymmetric encryption functionality
#[async_trait]
pub trait AsymmetricVault {
    /// Compute Elliptic-Curve Diffie-Hellman using this secret key
    /// and the specified uncompressed public key
    async fn ec_diffie_hellman(
        &mut self,
        context: &Secret,
        peer_public_key: &PublicKey,
    ) -> Result<Secret>;
}
