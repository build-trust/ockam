use crate::identities::{
    Identities, IdentitiesRepository, IdentitiesStorage, IdentitiesVault, Storage,
};
use ockam_core::compat::sync::Arc;
use ockam_core::vault::storage::Storage as VaultStorage;
use ockam_vault::Vault;

/// Builder for Identities services
#[derive(Clone)]
pub struct IdentitiesBuilder {
    vault: Arc<dyn IdentitiesVault>,
    repository: Arc<dyn IdentitiesRepository>,
}

/// Return a default identities
pub fn identities() -> Arc<Identities> {
    builder().build()
}

/// Return a default builder for identities
pub fn builder() -> IdentitiesBuilder {
    IdentitiesBuilder {
        vault: Vault::create(),
        repository: IdentitiesStorage::create(),
    }
}

impl IdentitiesBuilder {
    /// Set a specific storage for the identities vault
    pub fn with_vault_storage(&mut self, storage: Arc<dyn VaultStorage>) -> IdentitiesBuilder {
        self.with_identities_vault(Arc::new(Vault::new(Some(storage))))
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

    /// Set a specific repository
    pub fn with_identities_repository(
        &mut self,
        repository: Arc<dyn IdentitiesRepository>,
    ) -> IdentitiesBuilder {
        self.repository = repository;
        self.clone()
    }

    fn vault(&self) -> Arc<dyn IdentitiesVault> {
        self.vault.clone()
    }

    fn repository(&self) -> Arc<dyn IdentitiesRepository> {
        self.repository.clone()
    }

    /// Build identities
    pub fn build(&self) -> Arc<Identities> {
        Arc::new(Identities::new(self.vault(), self.repository()))
    }
}
