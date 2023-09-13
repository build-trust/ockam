use ockam_core::compat::sync::Arc;
use ockam_vault::{Vault, VaultStorage};

use super::super::identities::{Identities, IdentitiesRepository};
use super::super::secure_channel::SecureChannelRegistry;
use super::super::secure_channels::SecureChannels;
use super::super::storage::Storage;
use super::super::IdentitiesBuilder;

/// This struct supports all the services related to secure channels
#[derive(Clone)]
pub struct SecureChannelsBuilder {
    // FIXME: This is very strange dependency
    pub(crate) identities_builder: IdentitiesBuilder,
    pub(crate) registry: SecureChannelRegistry,
}

/// Create default, in-memory, secure channels (mostly for examples and testing)
pub fn secure_channels() -> Arc<SecureChannels> {
    SecureChannels::builder().build()
}

impl SecureChannelsBuilder {
    /// With Software Vault with given Storage
    pub fn with_vault_storage(mut self, storage: VaultStorage) -> Self {
        self.identities_builder = self.identities_builder.with_vault_storage(storage);
        self
    }

    /// Set [`Vault`]
    pub fn with_vault(mut self, vault: Vault) -> Self {
        self.identities_builder = self.identities_builder.with_vault(vault);
        self
    }

    /// Set a specific storage for the identities repository
    pub fn with_identities_storage(mut self, storage: Arc<dyn Storage>) -> Self {
        self.identities_builder = self.identities_builder.with_identities_storage(storage);
        self
    }

    /// Set a specific identities repository
    pub fn with_identities_repository(mut self, repository: Arc<dyn IdentitiesRepository>) -> Self {
        self.identities_builder = self
            .identities_builder
            .with_identities_repository(repository);
        self
    }

    /// Set a specific identities
    pub fn with_identities(mut self, identities: Arc<Identities>) -> Self {
        self.identities_builder = self
            .identities_builder
            .with_identities_repository(identities.repository())
            .with_vault(identities.vault())
            .with_purpose_keys_repository(identities.purpose_keys_repository());
        self
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
    pub fn build(self) -> Arc<SecureChannels> {
        let identities = self.identities_builder.build();
        let purpose_keys = identities.purpose_keys();
        Arc::new(SecureChannels::new(
            identities,
            purpose_keys,
            self.registry.clone(),
        ))
    }
}
