use super::super::identities::{IdentitiesKeys, IdentitiesRepository};
use super::super::purpose_keys::storage::{PurposeKeysRepository, PurposeKeysStorage};
use super::super::{
    Credentials, CredentialsServer, CredentialsServerModule, IdentitiesBuilder, IdentitiesCreation,
    IdentitiesReader, IdentitiesStorage, PurposeKeys,
};

use ockam_core::compat::sync::Arc;
use ockam_vault::Vault;

/// This struct supports all the services related to identities
#[derive(Clone)]
pub struct Identities {
    vault: Vault,
    identities_repository: Arc<dyn IdentitiesRepository>,
    purpose_keys_repository: Arc<dyn PurposeKeysRepository>,
}

impl Identities {
    /// Vault
    pub fn vault(&self) -> Vault {
        self.vault.clone()
    }

    /// Return the identities repository
    pub fn repository(&self) -> Arc<dyn IdentitiesRepository> {
        self.identities_repository.clone()
    }

    /// Return the purpose keys repository
    pub fn purpose_keys_repository(&self) -> Arc<dyn PurposeKeysRepository> {
        self.purpose_keys_repository.clone()
    }

    /// Return the [`PurposeKeys`] instance
    pub fn purpose_keys(&self) -> Arc<PurposeKeys> {
        Arc::new(PurposeKeys::new(
            self.vault.clone(),
            self.identities_repository.as_identities_reader(),
            self.identities_keys(),
            self.purpose_keys_repository.clone(),
        ))
    }

    /// Return the identities keys management service
    pub fn identities_keys(&self) -> Arc<IdentitiesKeys> {
        Arc::new(IdentitiesKeys::new(
            self.vault.signing_vault.clone(),
            self.vault.verifying_vault.clone(),
        ))
    }

    /// Return the identities creation service
    pub fn identities_creation(&self) -> Arc<IdentitiesCreation> {
        Arc::new(IdentitiesCreation::new(
            self.repository(),
            self.vault.signing_vault.clone(),
            self.vault.verifying_vault.clone(),
        ))
    }

    /// Return the identities reader
    pub fn identities_reader(&self) -> Arc<dyn IdentitiesReader> {
        self.repository().as_identities_reader()
    }

    /// Return the identities credentials service
    pub fn credentials(&self) -> Arc<Credentials> {
        Arc::new(Credentials::new(
            self.vault.signing_vault.clone(),
            self.vault.verifying_vault.clone(),
            self.purpose_keys(),
            self.identities_repository.clone(),
        ))
    }

    /// Return the identities credentials server
    pub fn credentials_server(&self) -> Arc<dyn CredentialsServer> {
        Arc::new(CredentialsServerModule::new(self.credentials()))
    }
}

impl Identities {
    /// Create a new identities module
    pub(crate) fn new(
        vault: Vault,
        identities_repository: Arc<dyn IdentitiesRepository>,
        purpose_keys_repository: Arc<dyn PurposeKeysRepository>,
    ) -> Identities {
        Identities {
            vault,
            identities_repository,
            purpose_keys_repository,
        }
    }

    /// Return a default builder for identities
    pub fn builder() -> IdentitiesBuilder {
        IdentitiesBuilder {
            vault: Vault::create(),
            repository: IdentitiesStorage::create(),
            purpose_keys_repository: PurposeKeysStorage::create(),
        }
    }
}
