use crate::{
    Changes, EventIdentifier, KeyAttributes, OckamError, Profile, ProfileChange,
    ProfileChangeEvent, ProfileChangeProof, ProfileChangeType, ProfileEventAttributes,
    ProfileVault, Signature, SignatureType,
};
use ockam_vault_core::{
    Secret, SecretAttributes, SecretPersistence, SecretType, CURVE25519_SECRET_LENGTH,
};
use serde::{Deserialize, Serialize};
use serde_big_array::big_array;
use std::ops::Deref;

big_array! { BigArray; }

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RotateKeyChangeData {
    key_attributes: KeyAttributes,
    public_key: Vec<u8>,
}

impl RotateKeyChangeData {
    pub fn key_attributes(&self) -> &KeyAttributes {
        &self.key_attributes
    }
    pub fn public_key(&self) -> &[u8] {
        self.public_key.as_slice()
    }
}

impl RotateKeyChangeData {
    pub fn new(key_attributes: KeyAttributes, public_key: Vec<u8>) -> Self {
        RotateKeyChangeData {
            key_attributes,
            public_key,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RotateKeyChange {
    data: RotateKeyChangeData,
    #[serde(with = "BigArray")]
    self_signature: [u8; 64],
    #[serde(with = "BigArray")]
    prev_signature: [u8; 64],
}

impl RotateKeyChange {
    pub fn data(&self) -> &RotateKeyChangeData {
        &self.data
    }
    pub fn self_signature(&self) -> &[u8; 64] {
        &self.self_signature
    }
    pub fn prev_signature(&self) -> &[u8; 64] {
        &self.prev_signature
    }
}

impl RotateKeyChange {
    pub fn new(
        data: RotateKeyChangeData,
        self_signature: [u8; 64],
        prev_signature: [u8; 64],
    ) -> Self {
        RotateKeyChange {
            data,
            self_signature,
            prev_signature,
        }
    }
}

impl Profile {
    pub(crate) fn rotate_key_event(
        &self,
        key_attributes: KeyAttributes,
        attributes: Option<ProfileEventAttributes>,
        root_key: &Secret,
        vault: &mut dyn ProfileVault,
    ) -> ockam_core::Result<ProfileChangeEvent> {
        let attributes = attributes.unwrap_or_default();

        let prev_event_id = self.change_history.get_last_event_id()?;

        let last_event_in_chain = self.change_history.find_last_key_event(&key_attributes)?;

        let last_key_in_chain =
            Self::get_secret_key_from_event(&key_attributes, last_event_in_chain, vault.deref())?;

        // TODO: Should be customisable
        let secret_attributes = SecretAttributes::new(
            SecretType::Curve25519,
            SecretPersistence::Persistent,
            CURVE25519_SECRET_LENGTH,
        );

        let secret_key = vault.secret_generate(secret_attributes)?;
        let public_key = vault.secret_public_key_get(&secret_key)?.as_ref().to_vec();

        let data = RotateKeyChangeData::new(key_attributes, public_key);
        let data_binary = serde_bare::to_vec(&data).map_err(|_| OckamError::BareError)?;
        let data_hash = vault.sha256(data_binary.as_slice())?;
        let self_signature = vault.sign(&secret_key, &data_hash)?;
        let prev_signature = vault.sign(&last_key_in_chain, &data_hash)?;
        let change = RotateKeyChange::new(data, self_signature, prev_signature);

        let profile_change = ProfileChange::new(
            Profile::CHANGE_CURRENT_VERSION,
            attributes.clone(),
            ProfileChangeType::RotateKey(change),
        );
        let changes = Changes::new(prev_event_id, vec![profile_change]);
        let changes_binary = serde_bare::to_vec(&changes).map_err(|_| OckamError::BareError)?;

        let event_id = vault.sha256(&changes_binary)?;
        let event_id = EventIdentifier::from_hash(event_id);

        let signature = vault.sign(root_key, event_id.as_ref())?;

        // TODO: Find root key and sign with it
        let proof =
            ProfileChangeProof::Signature(Signature::new(SignatureType::RootSign, signature));
        let signed_change_event = ProfileChangeEvent::new(event_id, changes, proof);

        Ok(signed_change_event)
    }
}
