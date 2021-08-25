use crate::change_history::ProfileChangeHistory;
use crate::profile::Profile;
use crate::{
    ChangeSet, EntityError, EventIdentifier, KeyAttributes, MetaKeyAttributes, ProfileChange,
    ProfileChangeEvent, ProfileChangeProof, ProfileChangeType, ProfileEventAttributes,
    ProfileState, Signature, SignatureType,
};
use ockam_core::compat::vec::Vec;
use ockam_vault::ockam_vault_core::Signature as OckamVaultSignature;
use ockam_vault::ockam_vault_core::{Hasher, SecretVault, Signer};
use serde::{Deserialize, Serialize};

/// RotateKeyChangeData
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RotateKeyChangeData {
    key_attributes: KeyAttributes,
    public_key: Vec<u8>,
}

impl RotateKeyChangeData {
    /// Return key attributes
    pub fn key_attributes(&self) -> &KeyAttributes {
        &self.key_attributes
    }
    /// Return public key
    pub fn public_key(&self) -> &[u8] {
        self.public_key.as_slice()
    }
}

impl RotateKeyChangeData {
    /// Create RotateKeyChangeData
    pub fn new(key_attributes: KeyAttributes, public_key: Vec<u8>) -> Self {
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
    pub(crate) fn rotate_key(
        &mut self,
        key_attributes: KeyAttributes,
        attributes: ProfileEventAttributes,
    ) -> ockam_core::Result<ProfileChangeEvent> {
        let prev_event_id = self.change_history().get_last_event_id()?;

        let last_event_in_chain = ProfileChangeHistory::find_last_key_event(
            self.change_history().as_ref(),
            &key_attributes,
        )?
        .clone();

        let mut vault = self.vault();

        let last_key_in_chain =
            Self::get_secret_key_from_event(&key_attributes, &last_event_in_chain, &mut vault)?;

        let secret_attributes = match key_attributes.meta() {
            MetaKeyAttributes::SecretAttributes(secret_attributes) => *secret_attributes,
            _ => panic!("missing secret attributes"),
        };

        let secret_key = vault.secret_generate(secret_attributes)?;
        let public_key = vault.secret_public_key_get(&secret_key)?.as_ref().to_vec();

        let data = RotateKeyChangeData::new(key_attributes, public_key);
        let data_binary = serde_bare::to_vec(&data).map_err(|_| EntityError::BareError)?;
        let data_hash = vault.sha256(data_binary.as_slice())?;
        let self_signature = vault.sign(&secret_key, &data_hash)?;
        let prev_signature = vault.sign(&last_key_in_chain, &data_hash)?;
        let change = RotateKeyChange::new(data, self_signature, prev_signature);

        let profile_change = ProfileChange::new(
            Profile::CURRENT_CHANGE_VERSION,
            attributes,
            ProfileChangeType::RotateKey(change),
        );
        let changes = ChangeSet::new(prev_event_id, vec![profile_change]);
        let changes_binary = serde_bare::to_vec(&changes).map_err(|_| EntityError::BareError)?;

        let event_id = vault.sha256(&changes_binary)?;
        let event_id = EventIdentifier::from_hash(event_id);

        let root_key = self.get_root_secret()?;

        let signature = vault.sign(&root_key, event_id.as_ref())?;

        let proof =
            ProfileChangeProof::Signature(Signature::new(SignatureType::RootSign, signature));
        let signed_change_event = ProfileChangeEvent::new(event_id, changes, proof);

        Ok(signed_change_event)
    }
}
