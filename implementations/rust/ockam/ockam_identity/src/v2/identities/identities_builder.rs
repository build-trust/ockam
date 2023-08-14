use super::super::identities::{
    Identities, IdentitiesRepository, IdentitiesStorage, IdentitiesVault,
};
use super::super::purpose_keys::storage::{PurposeKeysRepository, PurposeKeysStorage};
use super::super::storage::Storage;

use ockam_core::compat::sync::Arc;
use ockam_vault::{Vault, VaultStorage};

/// Builder for Identities services
#[derive(Clone)]
pub struct IdentitiesBuilder {
    pub(crate) vault: Arc<dyn IdentitiesVault>,
    pub(crate) repository: Arc<dyn IdentitiesRepository>,
    pub(crate) purpose_keys_repository: Arc<dyn PurposeKeysRepository>,
}

/// Return a default identities
pub fn identities() -> Arc<Identities> {
    Identities::builder().build()
}

impl IdentitiesBuilder {
    /// Set a specific storage for the identities vault
    pub fn with_vault_storage(&mut self, storage: VaultStorage) -> IdentitiesBuilder {
        self.with_identities_vault(Vault::create_with_persistent_storage(storage))
    }

    /// Set a specific identities vault
    pub fn with_identities_vault(&mut self, vault: Arc<dyn IdentitiesVault>) -> IdentitiesBuilder {
        self.vault = vault;
        self.clone()
    }

    /// Set a specific storage for identities
    pub fn with_identities_storage(&mut self, storage: Arc<dyn Storage>) -> IdentitiesBuilder {
        self.with_identities_repository(Arc::new(IdentitiesStorage::new(storage)))
    }

    /// Set a specific repository for identities
    pub fn with_identities_repository(
        &mut self,
        repository: Arc<dyn IdentitiesRepository>,
    ) -> IdentitiesBuilder {
        self.repository = repository;
        self.clone()
    }

    /// Set a specific storage for Purpose Keys
    pub fn with_purpose_keys_storage(&mut self, storage: Arc<dyn Storage>) -> IdentitiesBuilder {
        self.with_purpose_keys_repository(Arc::new(PurposeKeysStorage::new(storage)))
    }

    /// Set a specific repository for Purpose Keys
    pub fn with_purpose_keys_repository(
        &mut self,
        repository: Arc<dyn PurposeKeysRepository>,
    ) -> IdentitiesBuilder {
        self.purpose_keys_repository = repository;
        self.clone()
    }

    /// Build identities
    pub fn build(self) -> Arc<Identities> {
        Arc::new(Identities::new(
            self.vault,
            self.repository,
            self.purpose_keys_repository,
        ))
    }
}
