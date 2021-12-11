use crate::change_history::ProfileChangeHistory;
use crate::profile::Profile;
use crate::{
    ChangeBlock, EntityError, EventIdentifier, KeyAttributes, MetaKeyAttributes, ProfileChange,
    ProfileChangeEvent, ProfileChangeType, ProfileEventAttributes, ProfileState, Signature,
    SignatureType,
};
use ockam_core::vault::PublicKey;
use ockam_core::vault::Signature as OckamVaultSignature;
use ockam_core::vault::{Hasher, SecretVault, Signer};
use ockam_core::{Encodable, Result};
use serde::{Deserialize, Serialize};

/// RotateKeyChangeData
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RotateKeyChangeData {
    key_attributes: KeyAttributes,
    public_key: PublicKey,
}

impl RotateKeyChangeData {
    /// Return key attributes
    pub fn key_attributes(&self) -> &KeyAttributes {
        &self.key_attributes
    }
    /// Return public key
    pub fn public_key(&self) -> &PublicKey {
        &self.public_key
    }
}

impl RotateKeyChangeData {
    /// Create RotateKeyChangeData
    pub fn new(key_attributes: KeyAttributes, public_key: PublicKey) -> Self {
        RotateKeyChangeData {
            key_attributes,
            public_key,
        }
    }
}

/// RotateKeyChange
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RotateKeyChange {
    data: RotateKeyChangeData,
    self_signature: OckamVaultSignature,
    prev_signature: OckamVaultSignature,
}

impl RotateKeyChange {
    /// Return the data
    pub fn data(&self) -> &RotateKeyChangeData {
        &self.data
    }
    /// Return the self signature
    pub fn self_signature(&self) -> &OckamVaultSignature {
        &self.self_signature
    }
    /// Return the previous signature
    pub fn prev_signature(&self) -> &OckamVaultSignature {
        &self.prev_signature
    }
}

impl RotateKeyChange {
    /// Create a new RotateKeyChange
    pub fn new(
        data: RotateKeyChangeData,
        self_signature: OckamVaultSignature,
        prev_signature: OckamVaultSignature,
    ) -> Self {
        RotateKeyChange {
            data,
            self_signature,
            prev_signature,
        }
    }
}

impl ProfileState {
    /// Rotate key event
    pub(crate) async fn make_rotate_key_event(
        &mut self,
        key_attributes: KeyAttributes,
        attributes: ProfileEventAttributes,
    ) -> Result<ProfileChangeEvent> {
        let prev_event_id = self.change_history().get_last_event_id()?;

        let last_event_in_chain = ProfileChangeHistory::find_last_key_event(
            self.change_history().as_ref(),
            key_attributes.label(),
        )?
        .clone();

        let last_key_in_chain =
            Self::get_secret_key_from_event(&last_event_in_chain, &mut self.vault).await?;

        let secret_attributes = match key_attributes.meta() {
            MetaKeyAttributes::SecretAttributes(secret_attributes) => *secret_attributes,
        };

        let secret_key = self.vault.secret_generate(secret_attributes).await?;
        let public_key = self.vault.secret_public_key_get(&secret_key).await?;

        let data = RotateKeyChangeData::new(key_attributes, public_key);
        let data_binary = data.encode().map_err(|_| EntityError::BareError)?;
        let data_hash = self.vault.sha256(data_binary.as_slice()).await?;
        let self_signature = self.vault.sign(&secret_key, &data_hash).await?;
        let prev_signature = self.vault.sign(&last_key_in_chain, &data_hash).await?;
        let change = RotateKeyChange::new(data, self_signature, prev_signature);

        let profile_change = ProfileChange::new(
            Profile::CURRENT_CHANGE_VERSION,
            attributes,
            ProfileChangeType::RotateKey(change),
        );
        let change_block = ChangeBlock::new(prev_event_id, profile_change);
        let change_block_binary = change_block.encode().map_err(|_| EntityError::BareError)?;

        let event_id = self.vault.sha256(&change_block_binary).await?;
        let event_id = EventIdentifier::from_hash(event_id);

        let self_signature = self.vault.sign(&secret_key, event_id.as_ref()).await?;
        let self_signature = Signature::new(SignatureType::SelfSign, self_signature);

        let root_key = self.get_root_secret_key().await?;

        let root_signature = self.vault.sign(&root_key, event_id.as_ref()).await?;
        let root_signature = Signature::new(SignatureType::RootSign, root_signature);

        let signed_change_event =
            ProfileChangeEvent::new(event_id, change_block, vec![self_signature, root_signature]);

        Ok(signed_change_event)
    }
}
