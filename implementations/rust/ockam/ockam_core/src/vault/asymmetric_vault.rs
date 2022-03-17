use crate::vault::{PublicKey, Secret};
use crate::Result;
use crate::{async_trait, compat::boxed::Box};

/// Defines the Vault interface for asymmetric encryption.
#[async_trait]
pub trait AsymmetricVault {
    /// Compute Elliptic-Curve Diffie-Hellman using this secret key
    /// and the specified uncompressed public key.
    async fn ec_diffie_hellman(
        &self,
        context: &Secret,
        peer_public_key: &PublicKey,
    ) -> Result<Secret>;
}
