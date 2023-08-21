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

    async fn store_imported_identity(&self, identity: &Identity) -> Result<()> {
        // TODO: Duplicate
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

    /// Import and verify identity from its binary format
    /// This action persists the Identity in the storage, use `Identity::import` to avoid that
    pub async fn import(
        &self,
        expected_identifier: Option<&Identifier>,
        data: &[u8],
    ) -> Result<Identity> {
        let identity =
            Identity::import(expected_identifier, data, self.verifying_vault.clone()).await?;

        self.store_imported_identity(&identity).await?;

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

        self.store_imported_identity(&identity).await?;

        Ok(identity)
    }

    /// Create an `Identity`
    pub async fn create_identity(&self) -> Result<Identity> {
        // TODO: Consider creating PurposeKeys by default
        self.make_and_persist_identity(None).await
    }

    /// Create an `Identity` with a key previously created in the Vault
    pub async fn create_identity_with_existing_key(&self, kid: &KeyId) -> Result<Identity> {
        // TODO: Consider creating PurposeKeys by default
        self.make_and_persist_identity(Some(kid)).await
    }

    /// Create an identity with a vault initialized with a secret key
    /// encoded as a hex string.
    /// Such a key can be obtained by running vault.secret_export and then encoding
    /// the exported secret as a hex string
    /// Note: the data is not persisted!
    pub async fn import_private_identity(
        &self,
        identity_change_history: &[u8],
        key_id: &KeyId,
    ) -> Result<Identity> {
        let identity = self.import(None, identity_change_history).await?;
        if identity.get_public_key()? != self.signing_vault.get_public_key(key_id).await? {
            return Err(IdentityError::WrongSecretKey.into());
        }

        self.repository
            .update_identity(identity.identifier(), identity.change_history())
            .await?;
        Ok(identity)
    }
}

impl IdentitiesCreation {
    /// Make a new identity with its key and attributes
    /// and persist it
    async fn make_and_persist_identity(&self, key_id: Option<&KeyId>) -> Result<Identity> {
        let identity_keys =
            IdentitiesKeys::new(self.signing_vault.clone(), self.verifying_vault.clone());
        let identity = identity_keys.create_initial_key(key_id).await?;
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
}
