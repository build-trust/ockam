use ockam_core::compat::sync::Arc;
use ockam_core::compat::vec::Vec;
use ockam_core::Result;
use ockam_vault::{KeyId, Secret, SecretAttributes};

use crate::alloc::string::ToString;
use crate::identity::IdentityError;
use crate::{
    IdentitiesKeys, IdentitiesRepository, IdentitiesVault, Identity, IdentityChangeConstants,
    IdentityChangeHistory, IdentityIdentifier, KeyAttributes,
};

/// This struct supports functions for the creation and import of identities using an IdentityVault
pub struct IdentitiesCreation {
    repository: Arc<dyn IdentitiesRepository>,
    vault: Arc<dyn IdentitiesVault>,
}

impl IdentitiesCreation {
    /// Create a new identities import module
    pub fn new(
        repository: Arc<dyn IdentitiesRepository>,
        vault: Arc<dyn IdentitiesVault>,
    ) -> IdentitiesCreation {
        IdentitiesCreation { repository, vault }
    }

    /// Import and verify an `Identity` from its change history in a hex format
    pub async fn decode_identity_hex(&self, data: &str) -> Result<Identity> {
        self.decode_identity(
            hex::decode(data)
                .map_err(|_| IdentityError::ConsistencyError)?
                .as_slice(),
        )
        .await
    }

    /// Import and verify an `Identity` from its change history in a binary format
    pub async fn decode_identity(&self, data: &[u8]) -> Result<Identity> {
        let change_history = IdentityChangeHistory::import(data)?;
        let identity_keys = IdentitiesKeys::new(self.vault.clone());
        identity_keys
            .verify_all_existing_changes(&change_history)
            .await?;

        let identifier = self.compute_identity_identifier(&change_history).await?;
        Ok(Identity::new(identifier, change_history))
    }

    /// Create an identity with a vault initialized with a secret key
    /// encoded as a hex string.
    /// Such a key can be obtained by running vault.secret_export and then encoding
    /// the exported secret as a hex string
    /// Note: the data is not persisted!
    pub async fn import_private_identity(
        &self,
        identity_history: &str,
        secret: &str,
    ) -> Result<Identity> {
        let secret = Secret::new(hex::decode(secret).unwrap());
        let key_attributes = KeyAttributes::default_with_label(IdentityChangeConstants::ROOT_LABEL);
        self.vault
            .import_ephemeral_secret(secret, key_attributes.secret_attributes())
            .await?;
        let identity_history_data: Vec<u8> =
            hex::decode(identity_history).map_err(|_| IdentityError::InvalidInternalState)?;
        let identity = self
            .decode_identity(identity_history_data.as_slice())
            .await?;
        self.repository.update_identity(&identity).await?;
        Ok(identity)
    }

    /// Cryptographically compute `IdentityIdentifier`
    pub(super) async fn compute_identity_identifier(
        &self,
        change_history: &IdentityChangeHistory,
    ) -> Result<IdentityIdentifier> {
        let root_public_key = change_history.get_first_root_public_key()?;
        Ok(IdentityIdentifier::from_public_key(&root_public_key))
    }

    /// Create an `Identity` with a key previously created in the Vault. Extended version
    pub async fn create_identity_with_existing_key(
        &self,
        kid: &KeyId,
        attrs: KeyAttributes,
    ) -> Result<Identity> {
        self.make_and_persist_identity(Some(kid), attrs).await
    }

    /// Create an Identity
    pub async fn create_identity(&self) -> Result<Identity> {
        let attrs = KeyAttributes::new(
            IdentityChangeConstants::ROOT_LABEL.to_string(),
            SecretAttributes::Ed25519,
        );
        self.make_and_persist_identity(None, attrs).await
    }
}

impl IdentitiesCreation {
    /// Make a new identity with its key and attributes
    /// and persist it
    async fn make_and_persist_identity(
        &self,
        key_id: Option<&KeyId>,
        key_attributes: KeyAttributes,
    ) -> Result<Identity> {
        let identity_keys = IdentitiesKeys::new(self.vault.clone());
        let change_history = identity_keys
            .create_initial_key(key_id, key_attributes.clone())
            .await?;
        let identifier = self.compute_identity_identifier(&change_history).await?;
        let identity = Identity::new(identifier, change_history);
        self.repository.update_identity(&identity).await?;
        Ok(identity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identities;

    #[tokio::test]
    async fn test_identity_creation() -> Result<()> {
        let identities = identities();
        let creation = identities.identities_creation();
        let repository = identities.repository();
        let keys = identities.identities_keys();

        let identity = creation.create_identity().await?;
        let actual = repository.get_identity(&identity.identifier()).await?;
        assert_eq!(
            actual,
            identity.clone(),
            "the identity can be retrieved from the repository"
        );

        let actual = repository.retrieve_identity(&identity.identifier()).await?;
        assert_eq!(
            actual,
            Some(identity.clone()),
            "the identity can be retrieved from the repository as an Option"
        );

        let missing = repository
            .retrieve_identity(&IdentityIdentifier::from_hex(
                "e92f183eb4c324804ef4d62962dea94cf095a265d4d28500c34e1a4e0d5ef638",
            ))
            .await?;
        assert_eq!(missing, None, "a missing identity returns None");

        let root_key = keys.get_secret_key(&identity, None).await;
        assert!(root_key.is_ok(), "there is a key for the created identity");

        Ok(())
    }
}
