use ockam_core::compat::sync::Arc;
use ockam_core::compat::vec::Vec;
use ockam_core::Result;
use ockam_vault::{KeyId, Secret, SecretAttributes};

use crate::alloc::string::ToString;
use crate::identity::IdentityError;
use crate::models::ChangeHistory;
use crate::{IdentitiesKeys, IdentitiesRepository, IdentitiesVault, Identity, IdentityConstants};

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

    // /// Import and verify an `Identity` from its change history in a hex format
    // pub async fn decode_identity_hex(&self, data: &str) -> Result<Identity> {
    //     self.decode_identity(
    //         hex::decode(data)
    //             .map_err(|_| IdentityError::ConsistencyError)?
    //             .as_slice(),
    //     )
    //     .await
    // }
    //
    // /// Import and verify an `Identity` from its change history in a binary format
    // pub async fn decode_identity(&self, data: &[u8]) -> Result<Identity> {
    //     let change_history = ChangeHistory::import(data)?;
    //     let identity_keys = IdentitiesKeys::new(self.vault.clone());
    //     identity_keys
    //         .verify_all_existing_changes(&change_history)
    //         .await?;
    //
    //     let identifier = self.compute_identity_identifier(&change_history).await?;
    //     Ok(Identity::new(identifier, change_history))
    // }

    // /// Create an identity with a vault initialized with a secret key
    // /// encoded as a hex string.
    // /// Such a key can be obtained by running vault.secret_export and then encoding
    // /// the exported secret as a hex string
    // /// Note: the data is not persisted!
    // pub async fn import_private_identity(
    //     &self,
    //     identity_history: &str,
    //     secret: &str,
    // ) -> Result<Identity> {
    //     let secret = Secret::new(hex::decode(secret).unwrap());
    //     let key_attributes = KeyAttributes::default_with_label(IdentityConstants::ROOT_LABEL);
    //     self.vault
    //         .import_ephemeral_secret(secret, key_attributes.secret_attributes())
    //         .await?;
    //     let identity_history_data: Vec<u8> =
    //         hex::decode(identity_history).map_err(|_| IdentityError::InvalidInternalState)?;
    //     let identity = self
    //         .decode_identity(identity_history_data.as_slice())
    //         .await?;
    //     self.repository.update_identity(&identity).await?;
    //     Ok(identity)
    // }
    //
    // /// Cryptographically compute `Identifier`
    // pub(super) async fn compute_identity_identifier(
    //     &self,
    //     change_history: &ChangeHistory,
    // ) -> Result<Identifier> {
    //     let root_public_key = change_history.get_first_root_public_key()?;
    //     Ok(Identifier::from_public_key(&root_public_key))
    // }
    //
    // /// Create an `Identity` with a key previously created in the Vault. Extended version
    // pub async fn create_identity_with_existing_key(
    //     &self,
    //     kid: &KeyId,
    //     attrs: KeyAttributes,
    // ) -> Result<Identity> {
    //     self.make_and_persist_identity(Some(kid), attrs).await
    // }

    /// Create an Identity
    pub async fn create_identity(&self) -> Result<Identity> {
        self.make_and_persist_identity(None).await
    }
}

impl IdentitiesCreation {
    /// Make a new identity with its key and attributes
    /// and persist it
    async fn make_and_persist_identity(&self, key_id: Option<&KeyId>) -> Result<Identity> {
        let identity_keys = IdentitiesKeys::new(self.vault.clone());
        let identity = identity_keys.create_initial_key(key_id).await?;
        self.repository
            .update_identity(identity.identifier(), identity.change_history())
            .await?;
        Ok(identity)
    }
}
