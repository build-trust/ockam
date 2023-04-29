use crate::identities::{Identities, IdentitiesRepository, Storage};
use crate::secure_channel::SecureChannelRegistry;
use crate::secure_channels::SecureChannels;
use crate::{IdentitiesBuilder, IdentitiesVault};
use ockam_core::compat::sync::Arc;

/// This struct supports all the services related to secure channels
#[derive(Clone)]
pub struct SecureChannelsBuilder {
    pub(crate) identities_builder: IdentitiesBuilder,
    pub(crate) registry: SecureChannelRegistry,
}

/// Create default, in-memory, secure channels (mostly for examples and testing)
pub fn secure_channels() -> Arc<SecureChannels> {
    SecureChannels::builder().build()
}

impl SecureChannelsBuilder {
    /// Set a specific storage for the secure channels vault
    pub fn with_vault_storage(
        &mut self,
        storage: Arc<dyn ockam_core::vault::storage::Storage>,
    ) -> SecureChannelsBuilder {
        self.identities_builder = self.identities_builder.with_vault_storage(storage);
        self.clone()
    }

    /// Set a specific vault for secure channels
    pub fn with_identities_vault(
        &mut self,
        vault: Arc<dyn IdentitiesVault>,
    ) -> SecureChannelsBuilder {
        self.identities_builder = self.identities_builder.with_identities_vault(vault);
        self.clone()
    }

    /// Set a specific storage for the identities repository
    pub fn with_identities_storage(&mut self, storage: Arc<dyn Storage>) -> SecureChannelsBuilder {
        self.identities_builder = self.identities_builder.with_identities_storage(storage);
        self.clone()
    }

    /// Set a specific identities repository
    pub fn with_identities_repository(
        &mut self,
        repository: Arc<dyn IdentitiesRepository>,
    ) -> SecureChannelsBuilder {
        self.identities_builder = self
            .identities_builder
            .with_identities_repository(repository);
        self.clone()
    }

    /// Set a specific identities
    pub fn with_identities(&mut self, identities: Arc<Identities>) -> SecureChannelsBuilder {
        self.identities_builder = self
            .identities_builder
            .with_identities_repository(identities.repository())
            .with_identities_vault(identities.vault());
        self.clone()
    }

    /// Set a specific channel registry
    pub fn with_secure_channels_registry(
        &mut self,
        registry: SecureChannelRegistry,
    ) -> SecureChannelsBuilder {
        self.registry = registry;
        self.clone()
    }

    /// Return the vault used by this builder
    /// Build secure channels
    pub fn build(&self) -> Arc<SecureChannels> {
        Arc::new(SecureChannels::new(
            self.identities_builder.build(),
            SecureChannelRegistry::new(),
        ))
    }
}
