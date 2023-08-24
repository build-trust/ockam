use crate::IdentityHistoryComparison;
use ockam_core::compat::sync::Arc;
use ockam_core::Result;
use ockam_vault::{KeyId, SigningVault, VerifyingVault};

use super::super::models::{ChangeHistory, Identifier};
use super::super::{IdentitiesKeys, IdentitiesRepository, Identity, IdentityError};

/// This struct supports functions for the creation and import of identities using an IdentityVault
pub struct IdentitiesCreation {
    repository: Arc<dyn IdentitiesRepository>,
    signing_vault: Arc<dyn SigningVault>,
    verifying_vault: Arc<dyn VerifyingVault>,
}

impl IdentitiesCreation {
    /// Create a new identities import module
    pub fn new(
        repository: Arc<dyn IdentitiesRepository>,
        signing_vault: Arc<dyn SigningVault>,
        verifying_vault: Arc<dyn VerifyingVault>,
    ) -> Self {
        Self {
            repository,
            signing_vault,
            verifying_vault,
        }
    }

    /// Return the identities keys management service
    pub fn identities_keys(&self) -> Arc<IdentitiesKeys> {
        Arc::new(IdentitiesKeys::new(
            self.signing_vault.clone(),
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

    /// Create an `Identity` and store it
    pub async fn create_identity(&self) -> Result<Identity> {
        self.make_and_persist_identity(None).await
    }

    /// Rotate an existing `Identity` and update the stored version
    pub async fn rotate_identity(&self, identifier: Identifier) -> Result<()> {
        let change_history = self.repository.get_identity(&identifier).await?;

        let identity = Identity::import_from_change_history(
            Some(&identifier),
            change_history,
            self.verifying_vault.clone(),
        )
        .await?;

        let identity = self.identities_keys().rotate_key(identity).await?;

        self.repository
            .update_identity(identity.identifier(), identity.change_history())
            .await?;

        Ok(())
    }

    /// Create an `Identity` with a key previously created in the Vault and store it
    pub async fn create_identity_with_existing_key(&self, key_id: &KeyId) -> Result<Identity> {
        self.make_and_persist_identity(Some(key_id)).await
    }

    /// Import an existing Identity from its binary format
    /// Its secret is expected to exist in the Vault (either generated there, or some Vault
    /// implementations may allow importing a secret)
    pub async fn import_private_identity(
        &self,
        identity_change_history: &[u8],
        key_id: &KeyId,
    ) -> Result<Identity> {
        let identity = self.import(None, identity_change_history).await?;
        if identity.get_latest_public_key()? != self.signing_vault.get_public_key(key_id).await? {
            return Err(IdentityError::WrongSecretKey.into());
        }

        self.repository
            .update_identity(identity.identifier(), identity.change_history())
            .await?;
        Ok(identity)
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
    /// Make a new identity with its key and attributes
    /// and persist it
    async fn make_and_persist_identity(&self, key_id: Option<&KeyId>) -> Result<Identity> {
        let identity = self.identities_keys().create_initial_key(key_id).await?;
        self.repository
            .update_identity(identity.identifier(), identity.change_history())
            .await?;
        Ok(identity)
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::models::Identifier;
    use super::super::identities;
    use super::*;
    use core::str::FromStr;

    #[tokio::test]
    async fn test_identity_creation() -> Result<()> {
        let identities = identities();
        let creation = identities.identities_creation();
        let repository = identities.repository();
        let keys = identities.identities_keys();

        let identity = creation.create_identity().await?;
        let actual = repository.get_identity(identity.identifier()).await?;

        let actual = Identity::import_from_change_history(
            Some(identity.identifier()),
            actual,
            identities.vault().verifying_vault,
        )
        .await?;
        assert_eq!(
            actual, identity,
            "the identity can be retrieved from the repository"
        );

        let actual = repository.retrieve_identity(identity.identifier()).await?;
        assert!(actual.is_some());
        let actual = Identity::import_from_change_history(
            Some(identity.identifier()),
            actual.unwrap(),
            identities.vault().verifying_vault,
        )
        .await?;
        assert_eq!(
            actual, identity,
            "the identity can be retrieved from the repository as an Option"
        );

        let another_identifier = Identifier::from_str("Ie92f183eb4c324804ef4d62962dea94cf095a265")?;
        let missing = repository.retrieve_identity(&another_identifier).await?;
        assert_eq!(missing, None, "a missing identity returns None");

        let root_key = keys.get_secret_key(&identity).await;
        assert!(root_key.is_ok(), "there is a key for the created identity");

        Ok(())
    }

    // TODO TEST: rotation
}
