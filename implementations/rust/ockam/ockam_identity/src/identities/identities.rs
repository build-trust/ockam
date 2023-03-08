use crate::identities::{IdentitiesKeys, IdentitiesRepository, IdentitiesVault};
use crate::{Credentials, CredentialsServer, CredentialsServerModule, IdentitiesCreation};
use ockam_core::compat::sync::Arc;

impl Identities {
    /// Return the identities vault
    pub fn vault(&self) -> Arc<dyn IdentitiesVault> {
        self.vault.clone()
    }

    /// Return the identities repository
    pub fn repository(&self) -> Arc<dyn IdentitiesRepository> {
        self.identities_repository.clone()
    }

    /// Return the identities keys management service
    pub fn identities_keys(&self) -> Arc<IdentitiesKeys> {
        Arc::new(IdentitiesKeys::new(self.vault.clone()))
    }

    /// Return the identities creation service
    pub fn identities_creation(&self) -> Arc<IdentitiesCreation> {
        Arc::new(IdentitiesCreation::new(self.vault.clone()))
    }

    /// Return the identities credentials service
    pub fn credentials(&self) -> Arc<dyn Credentials> {
        Arc::new(self.clone())
    }

    /// Return the identities credentials server
    pub fn credentials_server(&self) -> Arc<dyn CredentialsServer> {
        Arc::new(CredentialsServerModule::new(self.credentials()))
    }
}

/// This struct supports all the services related to identities
#[derive(Clone)]
pub struct Identities {
    pub(crate) vault: Arc<dyn IdentitiesVault>,
    pub(crate) identities_repository: Arc<dyn IdentitiesRepository>,
}

impl Identities {
    /// Create a new identities module
    pub(crate) fn new(
        vault: Arc<dyn IdentitiesVault>,
        identities_repository: Arc<dyn IdentitiesRepository>,
    ) -> Identities {
        Identities {
            vault,
            identities_repository,
        }
    }
}
