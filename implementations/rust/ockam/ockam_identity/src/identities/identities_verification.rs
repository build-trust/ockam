use ockam_core::compat::sync::Arc;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{Error, Result};
use ockam_vault::VaultForVerifyingSignatures;

use crate::models::{ChangeHistory, Identifier};
use crate::{ChangeHistoryRepository, Identity};

/// This struct supports functions for the creation and import of identities using an IdentityVault
pub struct IdentitiesVerification {
    pub(super) repository: Arc<dyn ChangeHistoryRepository>,
    pub(super) verifying_vault: Arc<dyn VaultForVerifyingSignatures>,
}

impl IdentitiesVerification {
    /// Create a new identities import module
    pub fn new(
        repository: Arc<dyn ChangeHistoryRepository>,
        verifying_vault: Arc<dyn VaultForVerifyingSignatures>,
    ) -> Self {
        Self {
            repository,
            verifying_vault,
        }
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

    /// [`VerifyingVault`](ockam_vault::VaultForVerifyingSignatures)
    pub fn verifying_vault(&self) -> Arc<dyn VaultForVerifyingSignatures> {
        self.verifying_vault.clone()
    }

    /// Return the change history of a persisted identity
    pub async fn get_identity(&self, identifier: &Identifier) -> Result<Identity> {
        Self::get_identity_static(
            self.repository.clone(),
            self.verifying_vault.clone(),
            identifier,
        )
        .await
    }

    /// Return the change history of a persisted identity
    pub async fn get_identity_static(
        repository: Arc<dyn ChangeHistoryRepository>,
        verifying_vault: Arc<dyn VaultForVerifyingSignatures>,
        identifier: &Identifier,
    ) -> Result<Identity> {
        match repository.get_change_history(identifier).await? {
            Some(change_history) => {
                let identity = Identity::import_from_change_history(
                    Some(identifier),
                    change_history,
                    verifying_vault,
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

impl IdentitiesVerification {
    /// Compare Identity that was received by any side-channel (e.g., Secure Channel) to the
    /// version we have observed and stored before.
    ///   - Do nothing if they're equal
    ///   - Throw an error if the received version is older
    ///   - Throw an error if the received version has conflict
    ///   - Update stored Identity if the received version is newer
    ///
    /// All the code is performed in the ChangeHistoryRepository so that checking the identity
    /// new change history and the identity old change history + insert the new change history
    /// can be done atomically
    ///
    pub async fn update_identity(&self, identity: &Identity) -> Result<()> {
        self.repository.update_identity(identity, false).await
    }

    /// Compare Identity that was received by any side-channel (e.g., Secure Channel) to the
    /// version we have observed and stored before.
    ///   - Do nothing if they're equal
    ///   - Do nothing if the received version is older
    ///   - Throw an error if the received version has conflict
    ///   - Update stored Identity if the received version is newer
    ///
    /// All the code is performed in the ChangeHistoryRepository so that checking the identity
    /// new change history and the identity old change history + insert the new change history
    /// can be done atomically
    ///
    pub async fn update_identity_ignore_older(&self, identity: &Identity) -> Result<()> {
        self.repository.update_identity(identity, true).await
    }
}
