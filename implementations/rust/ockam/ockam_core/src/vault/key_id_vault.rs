use crate::vault::{KeyId, PublicKey, Secret};
use crate::Result;
use crate::{async_trait, compat::boxed::Box};

/// Defines the `KeyId` interface for Ockam vaults.
#[async_trait]
pub trait KeyIdVault {
    /// Return the `Secret` for a given key id.
    async fn get_secret_by_key_id(&mut self, key_id: &str) -> Result<Secret>;
    /// Compute and return the `KeyId` for a given public key.
    async fn compute_key_id_for_public_key(&mut self, public_key: &PublicKey) -> Result<KeyId>;
}
