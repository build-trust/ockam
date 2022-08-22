use crate::change_history::IdentityChangeHistory;
use crate::{IdentityError, IdentityIdentifier, IdentityVault};
use ockam_core::compat::{boxed::Box, vec::Vec};
use ockam_core::AsyncTryClone;
use ockam_core::Result;
use ockam_vault::PublicKey;

/// Identity implementation
#[derive(AsyncTryClone)]
#[async_try_clone(crate = "ockam_core")]
pub struct PublicIdentity<V: IdentityVault> {
    id: IdentityIdentifier,
    pub(crate) change_history: IdentityChangeHistory,
    pub(crate) vault: V,
}

impl<V: IdentityVault> PublicIdentity<V> {
    // Constructor
    pub fn new(id: IdentityIdentifier, change_history: IdentityChangeHistory, vault: V) -> Self {
        Self {
            id,
            change_history,
            vault,
        }
    }

    pub fn export(&self) -> Result<Vec<u8>> {
        self.change_history.export()
    }

    pub async fn import(data: &[u8], vault: &V) -> Result<Self> {
        let change_history = IdentityChangeHistory::import(data)?;
        if !change_history.verify_all_existing_events(vault).await? {
            return Err(IdentityError::IdentityVerificationFailed.into());
        }

        let id = change_history.compute_identity_id(vault).await?;

        let vault = vault.async_try_clone().await?;

        let identity = Self::new(id, change_history, vault);

        Ok(identity)
    }

    pub fn changes(&self) -> &IdentityChangeHistory {
        &self.change_history
    }

    pub fn vault(&self) -> &V {
        &self.vault
    }

    pub async fn verify_changes(&self) -> Result<bool> {
        self.change_history
            .verify_all_existing_events(&self.vault)
            .await
    }

    pub fn identifier(&self) -> &IdentityIdentifier {
        &self.id
    }

    pub fn get_root_public_key(&self) -> Result<PublicKey> {
        self.change_history.get_root_public_key()
    }

    pub fn get_public_key(&self, label: &str) -> Result<PublicKey> {
        self.change_history.get_public_key(label)
    }
}
