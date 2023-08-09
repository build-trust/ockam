use super::super::identities::{IdentitiesKeys, IdentitiesRepository, IdentitiesVault};
use super::super::purpose_keys::storage::PurposeKeysRepository;
use super::super::{
    Credentials, CredentialsServer, CredentialsServerModule, IdentitiesBuilder, IdentitiesCreation,
    IdentitiesReader, IdentitiesStorage, PurposeKeys,
};

use crate::v2::purpose_keys::storage::PurposeKeysStorage;
use ockam_core::compat::sync::Arc;
use ockam_vault::Vault;

/// This struct supports all the services related to identities
#[derive(Clone)]
pub struct Identities {
    vault: Arc<dyn IdentitiesVault>,
    identities_repository: Arc<dyn IdentitiesRepository>,
    purpose_keys_repository: Arc<dyn PurposeKeysRepository>,
}

impl Identities {
    /// Return the identities vault
    pub fn vault(&self) -> Arc<dyn IdentitiesVault> {
        self.vault.clone()
    }

    /// Return the identities repository
    pub fn repository(&self) -> Arc<dyn IdentitiesRepository> {
        self.identities_repository.clone()
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
        Arc::new(IdentitiesKeys::new(self.vault.clone()))
    }

    /// Return the identities creation service
    pub fn identities_creation(&self) -> Arc<IdentitiesCreation> {
        Arc::new(IdentitiesCreation::new(
            self.repository(),
            self.vault.clone(),
        ))
    }

    /// Return the identities reader
    pub fn identities_reader(&self) -> Arc<dyn IdentitiesReader> {
        self.repository().as_identities_reader()
    }

    /// Return the identities credentials service
    pub fn credentials(&self) -> Arc<Credentials> {
        Arc::new(Credentials::new(
            self.vault(),
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
        vault: Arc<dyn IdentitiesVault>,
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
        let vault = Vault::create();
        IdentitiesBuilder {
            vault: vault.clone(),
            repository: IdentitiesStorage::create(vault),
            purpose_keys_repository: PurposeKeysStorage::create(),
        }
    }
}
