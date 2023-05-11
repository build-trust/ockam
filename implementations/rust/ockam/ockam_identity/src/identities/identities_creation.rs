use ockam_core::compat::sync::Arc;
use ockam_core::compat::vec::Vec;
use ockam_core::vault::Secret::Key;
use ockam_core::vault::{
    KeyId, SecretAttributes, SecretKey, SecretPersistence, SecretType, CURVE25519_SECRET_LENGTH_U32,
};
use ockam_core::Result;

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
    pub async fn import_private_identity(
        &self,
        identity_history: &str,
        secret: &str,
    ) -> Result<Identity> {
        let key_attributes = KeyAttributes::default_with_label(IdentityChangeConstants::ROOT_LABEL);
        self.vault
            .secret_import(
                Key(SecretKey::new(hex::decode(secret).unwrap())),
                key_attributes.secret_attributes(),
            )
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
        let key_id = self
            .vault
            .compute_key_id_for_public_key(&root_public_key)
            .await?;

        Ok(IdentityIdentifier::from_key_id(&key_id))
    }

    /// Create an `Identity` with an external key. Extended version
    pub async fn create_identity_with_external_key(
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
            SecretAttributes::new(
                SecretType::Ed25519,
                SecretPersistence::Persistent,
                CURVE25519_SECRET_LENGTH_U32,
            ),
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
