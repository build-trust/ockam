use crate::{PublicKey, Secret};
use ockam_core::Result;
use zeroize::Zeroize;

/// Vault with asymmetric encryption functionality
pub trait AsymmetricVault: Zeroize {
    /// Compute Elliptic-Curve Diffie-Hellman using this secret key
    /// and the specified uncompressed public key
    fn ec_diffie_hellman(
        &mut self,
        context: &Secret,
        peer_public_key: &PublicKey,
    ) -> Result<Secret>;
}
