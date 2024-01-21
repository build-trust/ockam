use ockam_core::compat::sync::Arc;
#[cfg(feature = "storage")]
use ockam_core::Result;
#[cfg(feature = "storage")]
use ockam_node::database::SqlxDatabase;
use ockam_vault::storage::SecretsRepository;

use crate::identities::{ChangeHistoryRepository, Identities};
use crate::purpose_keys::storage::PurposeKeysRepository;
use crate::{IdentityAttributesRepository, Vault};

/// Builder for Identities services
#[derive(Clone)]
pub struct IdentitiesBuilder {
    pub(crate) vault: Vault,
    pub(crate) change_history_repository: Arc<dyn ChangeHistoryRepository>,
    pub(crate) identity_attributes_repository: Arc<dyn IdentityAttributesRepository>,
    pub(crate) purpose_keys_repository: Arc<dyn PurposeKeysRepository>,
}

/// Return a default identities
#[cfg(feature = "storage")]
pub async fn identities() -> Result<Arc<Identities>> {
    Ok(Identities::builder().await?.build())
}

/// Return identities backed by a specific database
#[cfg(feature = "storage")]
pub fn create(database: SqlxDatabase) -> Arc<Identities> {
    Identities::create(database).build()
}

impl IdentitiesBuilder {
    /// With Software Vault with given secrets repository
    pub fn with_secrets_repository(mut self, repository: Arc<dyn SecretsRepository>) -> Self {
        self.vault = Vault::create_with_secrets_repository(repository);
        self
    }

    /// Set a Vault
    pub fn with_vault(mut self, vault: Vault) -> Self {
        self.vault = vault;
        self
    }

    /// Set a specific repository for identities
    pub fn with_change_history_repository(
        mut self,
        repository: Arc<dyn ChangeHistoryRepository>,
    ) -> Self {
        self.change_history_repository = repository;
        self
    }

    /// Set a specific repository for identity attributes
    pub fn with_identity_attributes_repository(
        mut self,
        repository: Arc<dyn IdentityAttributesRepository>,
    ) -> Self {
        self.identity_attributes_repository = repository;
        self
    }

    /// Set a specific repository for Purpose Keys
    pub fn with_purpose_keys_repository(
        mut self,
        repository: Arc<dyn PurposeKeysRepository>,
    ) -> Self {
        self.purpose_keys_repository = repository;
        self
    }

    /// Build identities
    pub fn build(self) -> Arc<Identities> {
        Arc::new(Identities::new(
            self.vault,
            self.change_history_repository,
            self.identity_attributes_repository,
            self.purpose_keys_repository,
        ))
    }
}
