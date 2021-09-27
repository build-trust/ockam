use crate::{PublicKey, Secret};
use ockam_core::async_trait::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_core::Result;
use zeroize::Zeroize;

#[async_trait]
/// Vault with asymmetric encryption functionality
pub trait AsymmetricVault: Zeroize {
    /// Compute Elliptic-Curve Diffie-Hellman using this secret key
    /// and the specified uncompressed public key
    fn ec_diffie_hellman(
        &mut self,
        context: &Secret,
        peer_public_key: &PublicKey,
    ) -> Result<Secret>;

    /// Compute Elliptic-Curve Diffie-Hellman using this secret key
    /// and the specified uncompressed public key
    async fn async_ec_diffie_hellman(
        &mut self,
        context: &Secret,
        peer_public_key: &PublicKey,
    ) -> Result<Secret>;
}
