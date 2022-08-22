use crate::change::{
    ChangeBlock, IdentityChange, IdentityChangeEvent, IdentityChangeType, Signature, SignatureType,
};
use crate::change_history::IdentityChangeHistory;
use crate::IdentityError::InvalidInternalState;
use crate::{
    EventIdentifier, Identity, IdentityError, IdentityEventAttributes, IdentityStateConst,
    IdentityVault, KeyAttributes, MetaKeyAttributes,
};
use ockam_core::vault::Signature as OckamVaultSignature;
use ockam_core::vault::{KeyId, PublicKey};
use ockam_core::{Encodable, Result};
use serde::{Deserialize, Serialize};

/// Key change data creation
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateKeyChangeData {
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
}

impl CreateKeyChangeData {
    /// Create new CreateKeyChangeData
    pub fn new(key_attributes: KeyAttributes, public_key: PublicKey) -> Self {
        CreateKeyChangeData {
            key_attributes,
            public_key,
        }
    }
}

/// Key change creation
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateKeyChange {
    data: CreateKeyChangeData,
    self_signature: OckamVaultSignature,
}

impl CreateKeyChange {
    /// Return data
    pub fn data(&self) -> &CreateKeyChangeData {
        &self.data
    }
    /// Return self signature
    pub fn self_signature(&self) -> &OckamVaultSignature {
        &self.self_signature
    }
}

impl CreateKeyChange {
    /// Create new CreateKeyChange
    pub fn new(data: CreateKeyChangeData, self_signature: OckamVaultSignature) -> Self {
        CreateKeyChange {
            data,
            self_signature,
        }
    }
}

impl<V: IdentityVault> Identity<V> {
    async fn generate_key_if_needed(
        secret: Option<&KeyId>,
        key_attributes: &KeyAttributes,
        vault: &V,
    ) -> Result<KeyId> {
        if let Some(s) = secret {
            Ok(s.clone())
        } else {
            let MetaKeyAttributes::SecretAttributes(secret_attributes) = key_attributes.meta();

            vault.secret_generate(*secret_attributes).await
        }
    }

    /// Create a new key
    pub(crate) async fn make_create_key_event_static(
        secret: Option<&KeyId>,
        prev_id: EventIdentifier,
        key_attributes: KeyAttributes,
        attributes: IdentityEventAttributes,
        root_key: Option<&KeyId>,
        vault: &V,
    ) -> Result<IdentityChangeEvent> {
        let secret_key = Self::generate_key_if_needed(secret, &key_attributes, vault).await?;

        let public_key = vault.secret_public_key_get(&secret_key).await?;

        let data = CreateKeyChangeData::new(key_attributes, public_key);
        let data_binary = data.encode().map_err(|_| IdentityError::BareError)?;
        let data_hash = vault.sha256(data_binary.as_slice()).await?;
        let self_signature = vault.sign(&secret_key, &data_hash).await?;
        let change = CreateKeyChange::new(data, self_signature);

        let identity_change = IdentityChange::new(
            IdentityStateConst::CURRENT_CHANGE_VERSION,
            attributes,
            IdentityChangeType::CreateKey(change),
        );

        let change_block = ChangeBlock::new(prev_id, identity_change);
        let change_block_binary = change_block
            .encode()
            .map_err(|_| IdentityError::BareError)?;

        let event_id = vault.sha256(&change_block_binary).await?;
        let event_id = EventIdentifier::from_hash(event_id);

        let self_signature = vault.sign(&secret_key, event_id.as_ref()).await?;
        let self_signature = Signature::new(SignatureType::SelfSign, self_signature);

        let mut signatures = vec![self_signature];

        // If we have root_key passed we should sign using it
        // If there is no root_key - we're creating new identity, so we just generated root_key
        if let Some(root_key) = root_key {
            let root_signature = vault.sign(root_key, event_id.as_ref()).await?;
            let root_signature = Signature::new(SignatureType::RootSign, root_signature);

            signatures.push(root_signature);
        }

        let signed_change_event = IdentityChangeEvent::new(event_id, change_block, signatures);

        Ok(signed_change_event)
    }

    /// Create a new key
    pub(crate) async fn make_create_key_event(
        &self,
        secret: Option<&KeyId>,
        key_attributes: KeyAttributes,
        attributes: IdentityEventAttributes,
    ) -> Result<IdentityChangeEvent> {
        let change_history = self.change_history.read().await;
        // Creating key after it was revoked is forbidden
        if IdentityChangeHistory::find_last_key_event(
            change_history.as_ref(),
            key_attributes.label(),
        )
        .is_ok()
        {
            return Err(InvalidInternalState.into());
        }

        let prev_id = match change_history.get_last_event_id() {
            Ok(prev_id) => prev_id,
            Err(_) => EventIdentifier::initial(&self.vault).await,
        };

        let root_secret = self.get_root_secret_key().await?;
        let root_key = Some(&root_secret);

        Self::make_create_key_event_static(
            secret,
            prev_id,
            key_attributes,
            attributes,
            root_key,
            &self.vault,
        )
        .await
    }
}
