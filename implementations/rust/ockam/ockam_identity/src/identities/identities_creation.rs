use ockam_core::compat::sync::Arc;
use ockam_core::Result;
use ockam_vault::{SigningSecretKeyHandle, VaultForSigning, VaultForVerifyingSignatures};

use crate::identities::identity_builder::IdentityBuilder;
use crate::models::{ChangeHistory, Identifier};
use crate::{IdentitiesKeys, IdentitiesRepository, Identity, IdentityError};
use crate::{IdentityHistoryComparison, IdentityOptions};

/// This struct supports functions for the creation and import of identities using an IdentityVault
pub struct IdentitiesCreation {
    pub(super) repository: Arc<dyn IdentitiesRepository>,
    pub(super) identity_vault: Arc<dyn VaultForSigning>,
    pub(super) verifying_vault: Arc<dyn VaultForVerifyingSignatures>,
}

impl IdentitiesCreation {
    /// Create a new identities import module
    pub fn new(
        repository: Arc<dyn IdentitiesRepository>,
        identity_vault: Arc<dyn VaultForSigning>,
        verifying_vault: Arc<dyn VaultForVerifyingSignatures>,
    ) -> Self {
        Self {
            repository,
            identity_vault,
            verifying_vault,
        }
    }

    /// Return the identities keys management service
    pub fn identities_keys(&self) -> Arc<IdentitiesKeys> {
        Arc::new(IdentitiesKeys::new(
            self.identity_vault.clone(),
            self.verifying_vault.clone(),
        ))
    }

    /// Import and verify identity from its binary format
    /// This action persists the Identity in the storage, use `Identity::import` to avoid that
    pub async fn import(
        &self,
        expected_identifier: Option<&Identifier>,
        data: &[u8],
    ) -> Result<Identity> {
        let identity =
            Identity::import(expected_identifier, data, self.verifying_vault.clone()).await?;

        self.update_identity(&identity).await?;

        Ok(identity)
    }

    /// Import and verify identity from its Change History
    /// This action persists the Identity in the storage, use `Identity::import` to avoid that
    pub async fn import_from_change_history(
        &self,
        expected_identifier: Option<&Identifier>,
        change_history: ChangeHistory,
    ) -> Result<Identity> {
        let identity = Identity::import_from_change_history(
            expected_identifier,
            change_history,
            self.verifying_vault.clone(),
        )
        .await?;

        self.update_identity(&identity).await?;

        Ok(identity)
    }

    /// Get an instance of [`IdentityBuilder`]
    pub fn identity_builder(&self) -> IdentityBuilder {
        IdentityBuilder::new(Arc::new(Self::new(
            self.repository.clone(),
            self.identity_vault.clone(),
            self.verifying_vault.clone(),
        )))
    }

    /// Create an `Identity` and store it
    pub async fn create_identity(&self) -> Result<Identity> {
        let builder = self.identity_builder();
        builder.build().await
    }

    /// Create an `Identity` and store it
    pub async fn create_identity_with_options(&self, options: IdentityOptions) -> Result<Identity> {
        let identity = self.identities_keys().create_initial_key(options).await?;
        self.repository
            .update_identity(identity.identifier(), identity.change_history())
            .await?;
        Ok(identity)
    }

    /// Rotate an existing `Identity` and update the stored version
    pub async fn rotate_identity(&self, identifier: &Identifier) -> Result<()> {
        let builder = self.identity_builder();
        let options = builder.build_options().await?;

        self.rotate_identity_with_options(identifier, options).await
    }

    /// Rotate an existing `Identity` and update the stored version
    pub async fn rotate_identity_with_options(
        &self,
        identifier: &Identifier,
        options: IdentityOptions,
    ) -> Result<()> {
        let change_history = self.repository.get_identity(identifier).await?;

        let identity = Identity::import_from_change_history(
            Some(identifier),
            change_history,
            self.verifying_vault.clone(),
        )
        .await?;

        let identity = self
            .identities_keys()
            .rotate_key_with_options(identity, options)
            .await?;

        self.repository
            .update_identity(identity.identifier(), identity.change_history())
            .await?;

        Ok(())
    }

    /// Import an existing Identity from its binary format
    /// Its secret is expected to exist in the Vault (either generated there, or some Vault
    /// implementations may allow importing a secret)
    pub async fn import_private_identity(
        &self,
        identity_change_history: &[u8],
        signing_secret_key_handle: &SigningSecretKeyHandle,
    ) -> Result<Identity> {
        let identity = self.import(None, identity_change_history).await?;
        if identity.get_latest_public_key()?
            != self
                .identity_vault
                .get_verifying_public_key(signing_secret_key_handle)
                .await?
        {
            return Err(IdentityError::WrongSecretKey.into());
        }

        self.repository
            .update_identity(identity.identifier(), identity.change_history())
            .await?;
        Ok(identity)
    }

    /// [`SigningVault`]
    pub fn identity_vault(&self) -> Arc<dyn VaultForSigning> {
        self.identity_vault.clone()
    }

    /// [`VerifyingVault`]
    pub fn verifying_vault(&self) -> Arc<dyn VaultForVerifyingSignatures> {
        self.verifying_vault.clone()
    }
}

impl IdentitiesCreation {
    /// Compare Identity that was received by any side-channel (e.g., Secure Channel) to the
    /// version we have observed and stored before.
    ///   - Do nothing if they're equal
    ///   - Throw an error if the received version has conflict or is older that previously observed
    ///   - Update stored Identity if the received version is newer
    pub async fn update_identity(&self, identity: &Identity) -> Result<()> {
        if let Some(known_identity) = self
            .repository
            .retrieve_identity(identity.identifier())
            .await?
        {
            let known_identity = Identity::import_from_change_history(
                Some(identity.identifier()),
                known_identity,
                self.verifying_vault.clone(),
            )
            .await?;

            match identity.compare(&known_identity) {
                IdentityHistoryComparison::Conflict | IdentityHistoryComparison::Older => {
                    return Err(IdentityError::ConsistencyError.into());
                }
                IdentityHistoryComparison::Newer => {
                    self.repository
                        .update_identity(identity.identifier(), identity.change_history())
                        .await?;
                }
                IdentityHistoryComparison::Equal => {}
            }
        } else {
            self.repository
                .update_identity(identity.identifier(), identity.change_history())
                .await?;
        }

        Ok(())
    }
}
