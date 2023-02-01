use crate::authenticated_storage::AuthenticatedStorage;
use crate::change::{IdentityChange, IdentitySignedChange, Signature, SignatureType};
use crate::change_history::IdentityChangeHistory;
use crate::IdentityError::InvalidInternalState;
use crate::{ChangeIdentifier, Identity, IdentityError, IdentityVault, KeyAttributes};
use core::fmt;
use ockam_core::vault::{KeyId, PublicKey};
use ockam_core::{Encodable, Result};
use serde::{Deserialize, Serialize};

/// Key change data creation
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateKeyChangeData {
    prev_change_id: ChangeIdentifier,
    key_attributes: KeyAttributes,
    public_key: PublicKey,
}

impl CreateKeyChangeData {
    /// Return key attributes
    pub fn key_attributes(&self) -> &KeyAttributes {
        &self.key_attributes
    }
    /// Return public key
    pub fn public_key(&self) -> &PublicKey {
        &self.public_key
    }
    /// Previous change identifier, used to create a chain
    pub fn prev_change_id(&self) -> &ChangeIdentifier {
        &self.prev_change_id
    }
}

impl CreateKeyChangeData {
    /// Create new CreateKeyChangeData
    pub fn new(
        prev_change_id: ChangeIdentifier,
        key_attributes: KeyAttributes,
        public_key: PublicKey,
    ) -> Self {
        Self {
            prev_change_id,
            key_attributes,
            public_key,
        }
    }
}

impl fmt::Display for CreateKeyChangeData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "prev_change_id:{} key attibutes:{} public key:{}",
            self.prev_change_id(),
            self.key_attributes(),
            self.public_key()
        )
    }
}

impl<V: IdentityVault, S: AuthenticatedStorage> Identity<V, S> {
    async fn generate_key_if_needed(
        secret: Option<&KeyId>,
        key_attributes: &KeyAttributes,
        vault: &V,
    ) -> Result<KeyId> {
        if let Some(s) = secret {
            Ok(s.clone())
        } else {
            vault
                .secret_generate(key_attributes.secret_attributes())
                .await
        }
    }

    /// Create a new key
    pub(crate) async fn make_create_key_change_static(
        secret: Option<&KeyId>,
        prev_id: ChangeIdentifier,
        key_attributes: KeyAttributes,
        root_key: Option<&KeyId>,
        vault: &V,
    ) -> Result<IdentitySignedChange> {
        let secret_key = Self::generate_key_if_needed(secret, &key_attributes, vault).await?;

        let public_key = vault.secret_public_key_get(&secret_key).await?;

        let data = CreateKeyChangeData::new(prev_id, key_attributes, public_key);

        let change_block = IdentityChange::CreateKey(data);
        let change_block_binary = change_block
            .encode()
            .map_err(|_| IdentityError::BareError)?;

        let change_id = vault.sha256(&change_block_binary).await?;
        let change_id = ChangeIdentifier::from_hash(change_id);

        let self_signature = vault.sign(&secret_key, change_id.as_ref()).await?;
        let self_signature = Signature::new(SignatureType::SelfSign, self_signature);

        let mut signatures = vec![self_signature];

        // If we have root_key passed we should sign using it
        // If there is no root_key - we're creating new identity, so we just generated root_key
        if let Some(root_key) = root_key {
            let root_signature = vault.sign(root_key, change_id.as_ref()).await?;
            let root_signature = Signature::new(SignatureType::RootSign, root_signature);

            signatures.push(root_signature);
        }

        let signed_change = IdentitySignedChange::new(change_id, change_block, signatures);

        Ok(signed_change)
    }

    /// Create a new key
    pub(crate) async fn make_create_key_change(
        &self,
        secret: Option<&KeyId>,
        key_attributes: KeyAttributes,
    ) -> Result<IdentitySignedChange> {
        let change_history = self.change_history.read().await;
        // Creating key after it was revoked is forbidden
        if IdentityChangeHistory::find_last_key_change(
            change_history.as_ref(),
            key_attributes.label(),
        )
        .is_ok()
        {
            return Err(InvalidInternalState.into());
        }

        let prev_id = match change_history.get_last_change_id() {
            Ok(prev_id) => prev_id,
            Err(_) => ChangeIdentifier::initial(&self.vault).await,
        };

        let root_secret = self.get_root_secret_key().await?;
        let root_key = Some(&root_secret);

        Self::make_create_key_change_static(secret, prev_id, key_attributes, root_key, &self.vault)
            .await
    }
}
