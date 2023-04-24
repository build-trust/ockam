use crate::{KeyId, PublicKey};
use ockam_core::{async_trait, compat::boxed::Box, Result};

/// Defines the Vault interface for asymmetric encryption.
#[async_trait]
pub trait AsymmetricVault {
    /// Compute Elliptic-Curve Diffie-Hellman using this secret key
    /// and the specified uncompressed public key.
    async fn ec_diffie_hellman(&self, secret: &KeyId, peer_public_key: &PublicKey)
        -> Result<KeyId>;

    /// Compute and return the `KeyId` for a given public key.
    async fn compute_key_id_for_public_key(&self, public_key: &PublicKey) -> Result<KeyId>;
}
