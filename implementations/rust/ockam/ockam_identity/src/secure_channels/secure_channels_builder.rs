use ockam_core::compat::sync::Arc;
#[cfg(feature = "storage")]
use ockam_core::Result;
use ockam_vault::storage::SecretsRepository;

use crate::identities::{ChangeHistoryRepository, Identities};
use crate::purpose_keys::storage::PurposeKeysRepository;
use crate::secure_channel::SecureChannelRegistry;
use crate::secure_channels::SecureChannels;
use crate::{CredentialRepository, IdentitiesBuilder, IdentityAttributesRepository, Vault};

/// This struct supports all the services related to secure channels
#[derive(Clone)]
pub struct SecureChannelsBuilder {
    // FIXME: This is very strange dependency
    pub(crate) identities_builder: IdentitiesBuilder,
    pub(crate) registry: SecureChannelRegistry,
}

/// Create default, in-memory, secure channels (mostly for examples and testing)
#[cfg(feature = "storage")]
pub async fn secure_channels() -> Result<Arc<SecureChannels>> {
    Ok(SecureChannels::builder().await?.build())
}

impl SecureChannelsBuilder {
    /// With Software Vault with given secrets repository
    pub fn with_secrets_repository(mut self, repository: Arc<dyn SecretsRepository>) -> Self {
        self.identities_builder = self.identities_builder.with_secrets_repository(repository);
        self
    }

    /// Set [`Vault`]
    pub fn with_vault(mut self, vault: Vault) -> Self {
        self.identities_builder = self.identities_builder.with_vault(vault);
        self
    }

    /// Set a specific identities repository
    pub fn with_change_history_repository(
        mut self,
        repository: Arc<dyn ChangeHistoryRepository>,
    ) -> Self {
        self.identities_builder = self
            .identities_builder
            .with_change_history_repository(repository);
        self
    }

    /// Set a specific identity attributes repository
    pub fn with_identity_attributes_repository(
        mut self,
        repository: Arc<dyn IdentityAttributesRepository>,
    ) -> Self {
        self.identities_builder = self
            .identities_builder
            .with_identity_attributes_repository(repository);
        self
    }

    /// Set a specific purpose keys repository
    pub fn with_purpose_keys_repository(
        mut self,
        repository: Arc<dyn PurposeKeysRepository>,
    ) -> Self {
        self.identities_builder = self
            .identities_builder
            .with_purpose_keys_repository(repository);
        self
    }

    /// Set a specific cached credentials repository
    pub fn with_cached_credential_repository(
        mut self,
        repository: Arc<dyn CredentialRepository>,
    ) -> Self {
        self.identities_builder = self
            .identities_builder
            .with_cached_credential_repository(repository);
        self
    }

    /// Set a specific identities
    pub fn with_identities(mut self, identities: Arc<Identities>) -> Self {
        self.identities_builder = self
            .identities_builder
            .with_change_history_repository(identities.change_history_repository())
            .with_identity_attributes_repository(identities.identity_attributes_repository())
            .with_vault(identities.vault())
            .with_purpose_keys_repository(identities.purpose_keys_repository())
            .with_cached_credential_repository(identities.cached_credentials_repository());
        self
    }

    /// Set a specific channel registry
    pub fn with_secure_channels_registry(mut self, registry: SecureChannelRegistry) -> Self {
        self.registry = registry;
        self
    }

    /// Return the vault used by this builder
    /// Build secure channels
    pub fn build(self) -> Arc<SecureChannels> {
        let identities = self.identities_builder.build();
        Arc::new(SecureChannels::new(identities, self.registry.clone()))
    }
}
