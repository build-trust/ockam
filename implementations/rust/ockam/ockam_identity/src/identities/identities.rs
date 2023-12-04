use ockam_core::compat::sync::Arc;
use ockam_core::compat::vec::Vec;
use ockam_core::Result;
#[cfg(feature = "storage")]
use ockam_node::database::SqlxDatabase;

#[cfg(feature = "storage")]
use crate::identities::storage::ChangeHistorySqlxDatabase;
use crate::identities::storage::CredentialRepository;
#[cfg(feature = "storage")]
use crate::identities::storage::CredentialSqlxDatabase;
#[cfg(feature = "storage")]
use crate::identities::storage::IdentityAttributesSqlxDatabase;
use crate::identities::{ChangeHistoryRepository, IdentitiesKeys};
use crate::models::ChangeHistory;
use crate::purpose_keys::storage::PurposeKeysRepository;
#[cfg(feature = "storage")]
use crate::purpose_keys::storage::PurposeKeysSqlxDatabase;
#[cfg(feature = "storage")]
use crate::IdentitiesBuilder;
use crate::{
    Credentials, Identifier, IdentitiesCreation, IdentitiesVerification, Identity,
    IdentityAttributesRepository, PurposeKeys, Vault,
};

/// This struct supports all the services related to identities
#[derive(Clone)]
pub struct Identities {
    vault: Vault,
    change_history_repository: Arc<dyn ChangeHistoryRepository>,
    identity_attributes_repository: Arc<dyn IdentityAttributesRepository>,
    purpose_keys_repository: Arc<dyn PurposeKeysRepository>,
    cached_credentials_repository: Arc<dyn CredentialRepository>,
}

impl Identities {
    /// Vault
    pub fn vault(&self) -> Vault {
        self.vault.clone()
    }

    /// Return the identities repository
    pub fn change_history_repository(&self) -> Arc<dyn ChangeHistoryRepository> {
        self.change_history_repository.clone()
    }

    /// Return the identity attributes repository
    pub fn identity_attributes_repository(&self) -> Arc<dyn IdentityAttributesRepository> {
        self.identity_attributes_repository.clone()
    }

    /// Return the purpose keys repository
    pub fn purpose_keys_repository(&self) -> Arc<dyn PurposeKeysRepository> {
        self.purpose_keys_repository.clone()
    }

    /// Return the cached credentials repository
    pub fn cached_credentials_repository(&self) -> Arc<dyn CredentialRepository> {
        self.cached_credentials_repository.clone()
    }

    /// Get an [`Identity`] from the repository
    pub async fn get_identity(&self, identifier: &Identifier) -> Result<Identity> {
        self.identities_verification()
            .get_identity(identifier)
            .await
    }

    /// Return the change history of a persisted identity
    pub async fn get_change_history(&self, identifier: &Identifier) -> Result<ChangeHistory> {
        self.identities_verification()
            .get_change_history(identifier)
            .await
    }

    /// Export an [`Identity`] from the repository
    pub async fn export_identity(&self, identifier: &Identifier) -> Result<Vec<u8>> {
        self.get_identity(identifier).await?.export()
    }

    /// Return the [`PurposeKeys`] instance
    pub fn purpose_keys(&self) -> Arc<PurposeKeys> {
        Arc::new(PurposeKeys::new(
            self.vault.clone(),
            self.change_history_repository.clone(),
            self.identities_keys(),
            self.purpose_keys_repository.clone(),
        ))
    }

    /// Return the identities keys management service
    pub fn identities_keys(&self) -> Arc<IdentitiesKeys> {
        Arc::new(IdentitiesKeys::new(
            self.vault.identity_vault.clone(),
            self.vault.verifying_vault.clone(),
        ))
    }

    /// Return the identities creation service
    pub fn identities_creation(&self) -> Arc<IdentitiesCreation> {
        Arc::new(IdentitiesCreation::new(
            self.change_history_repository(),
            self.vault.identity_vault.clone(),
            self.vault.verifying_vault.clone(),
        ))
    }

    /// Return the identities verification service
    pub fn identities_verification(&self) -> Arc<IdentitiesVerification> {
        Arc::new(IdentitiesVerification::new(
            self.change_history_repository(),
            self.vault.verifying_vault.clone(),
        ))
    }

    /// Return the identities credentials service
    pub fn credentials(&self) -> Arc<Credentials> {
        Arc::new(Credentials::new(
            self.vault.credential_vault.clone(),
            self.vault.verifying_vault.clone(),
            self.purpose_keys(),
            self.identities_creation().clone(),
            self.identity_attributes_repository.clone(),
        ))
    }
}

impl Identities {
    /// Create a new identities module
    pub fn new(
        vault: Vault,
        change_history_repository: Arc<dyn ChangeHistoryRepository>,
        identity_attributes_repository: Arc<dyn IdentityAttributesRepository>,
        purpose_keys_repository: Arc<dyn PurposeKeysRepository>,
        cached_credentials_repository: Arc<dyn CredentialRepository>,
    ) -> Identities {
        Identities {
            vault,
            change_history_repository,
            identity_attributes_repository,
            purpose_keys_repository,
            cached_credentials_repository,
        }
    }

    /// Return a default builder for identities
    #[cfg(feature = "storage")]
    pub async fn builder() -> Result<IdentitiesBuilder> {
        Ok(Self::create(
            SqlxDatabase::in_memory("identities-builder").await?,
        ))
    }

    /// Return a builder for identities with a specific database
    #[cfg(feature = "storage")]
    pub fn create(database: SqlxDatabase) -> IdentitiesBuilder {
        IdentitiesBuilder {
            vault: Vault::create_with_database(database.clone()),
            change_history_repository: Arc::new(ChangeHistorySqlxDatabase::new(database.clone())),
            identity_attributes_repository: Arc::new(IdentityAttributesSqlxDatabase::new(
                database.clone(),
            )),
            purpose_keys_repository: Arc::new(PurposeKeysSqlxDatabase::new(database.clone())),
            cached_credentials_repository: Arc::new(CredentialSqlxDatabase::new(database)),
        }
    }
}
