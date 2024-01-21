use ockam_core::compat::sync::Arc;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{Error, Result};
use ockam_vault::{SigningSecretKeyHandle, VaultForSigning, VaultForVerifyingSignatures};

use crate::identities::identity_builder::IdentityBuilder;
use crate::models::{ChangeHistory, Identifier};
use crate::IdentityOptions;
use crate::{ChangeHistoryRepository, IdentitiesKeys, Identity, IdentityError};

/// This struct supports functions for the creation and import of identities using an IdentityVault
pub struct IdentitiesCreation {
    pub(super) repository: Arc<dyn ChangeHistoryRepository>,
    pub(super) identity_vault: Arc<dyn VaultForSigning>,
    pub(super) verifying_vault: Arc<dyn VaultForVerifyingSignatures>,
}

impl IdentitiesCreation {
    /// Create a new identities import module
    pub fn new(
        repository: Arc<dyn ChangeHistoryRepository>,
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
    pub async fn import(
        &self,
        expected_identifier: Option<&Identifier>,
        data: &[u8],
    ) -> Result<Identifier> {
        self.import_from_change_history(expected_identifier, ChangeHistory::import(data)?)
            .await
    }

    /// Import and verify identity from its Change History
    /// This action persists the Identity in the storage, use `Identity::import` to avoid that
    pub async fn import_from_change_history(
        &self,
        expected_identifier: Option<&Identifier>,
        change_history: ChangeHistory,
    ) -> Result<Identifier> {
        let identity = Identity::import_from_change_history(
            expected_identifier,
            change_history,
            self.verifying_vault.clone(),
        )
        .await?;

        self.update_identity(&identity).await?;
        Ok(identity.identifier().clone())
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
    pub async fn create_identity(&self) -> Result<Identifier> {
        let builder = self.identity_builder();
        builder.build().await
    }

    /// Create an `Identity` and store it
    pub async fn create_identity_with_options(
        &self,
        options: IdentityOptions,
    ) -> Result<Identifier> {
        let identity = self.identities_keys().create_initial_key(options).await?;
        self.repository
            .store_change_history(identity.identifier(), identity.change_history().clone())
            .await?;
        Ok(identity.identifier().clone())
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
        let identity = self.get_identity(identifier).await?;
        let identity = self
            .identities_keys()
            .rotate_key_with_options(identity, options)
            .await?;

        self.update_identity(&identity).await?;

        Ok(())
    }

    /// Import an existing Identity from its binary format
    /// Its secret is expected to exist in the Vault (either generated there, or some Vault
    /// implementations may allow importing a secret)
    pub async fn import_private_identity(
        &self,
        expected_identifier: Option<&Identifier>,
        identity_change_history: &[u8],
        signing_secret_key_handle: &SigningSecretKeyHandle,
    ) -> Result<Identifier> {
        let identity = Identity::import(
            expected_identifier,
            identity_change_history,
            self.verifying_vault.clone(),
        )
        .await?;

        self.update_identity(&identity).await?;

        if identity.get_latest_public_key()?
            != self
                .identity_vault
                .get_verifying_public_key(signing_secret_key_handle)
                .await?
        {
            return Err(IdentityError::WrongSecretKey)?;
        }
        Ok(identity.identifier().clone())
    }

    /// [`SigningVault`]
    pub fn identity_vault(&self) -> Arc<dyn VaultForSigning> {
        self.identity_vault.clone()
    }

    /// [`VerifyingVault`]
    pub fn verifying_vault(&self) -> Arc<dyn VaultForVerifyingSignatures> {
        self.verifying_vault.clone()
    }

    /// Return the change history of a persisted identity
    pub async fn get_identity(&self, identifier: &Identifier) -> Result<Identity> {
        match self.repository.get_change_history(identifier).await? {
            Some(change_history) => {
                let identity = Identity::import_from_change_history(
                    Some(identifier),
                    change_history,
                    self.verifying_vault.clone(),
                )
                .await?;
                Ok(identity)
            }
            None => Err(Error::new(
                Origin::Core,
                Kind::NotFound,
                format!("identity not found for identifier {}", identifier),
            )),
        }
    }
    /// Return the change history of a persisted identity
    pub async fn get_change_history(&self, identifier: &Identifier) -> Result<ChangeHistory> {
        match self.repository.get_change_history(identifier).await? {
            Some(change_history) => Ok(change_history),
            None => Err(Error::new(
                Origin::Core,
                Kind::NotFound,
                format!("identity not found for identifier {}", identifier),
            )),
        }
    }
}

impl IdentitiesCreation {
    /// Compare Identity that was received by any side-channel (e.g., Secure Channel) to the
    /// version we have observed and stored before.
    ///   - Do nothing if they're equal
    ///   - Throw an error if the received version has conflict or is older that previously observed
    ///   - Update stored Identity if the received version is newer
    ///
    /// All the code is performed in the ChangeHistoryRepository so that checking the identity
    /// new change history and the identity old change history + insert the new change history
    /// can be done atomically
    ///
    pub async fn update_identity(&self, identity: &Identity) -> Result<()> {
        self.repository.update_identity(identity).await
    }
}
