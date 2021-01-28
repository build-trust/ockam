use crate::{
    EventIdentifier, KeyAttributes, OckamError, Profile, ProfileChange, ProfileChangeEvent,
    ProfileChangeProof, ProfileChangeType, ProfileEventAttributes, ProfileVault, Signature,
    SignatureType, PROFILE_CHANGE_CURRENT_VERSION,
};
use ockam_vault_core::{
    Secret, SecretAttributes, SecretPersistence, SecretType, CURVE25519_SECRET_LENGTH,
};
use serde::{Deserialize, Serialize};
use serde_big_array::big_array;
use std::ops::DerefMut;

big_array! { BigArray; }

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateKeyChangeData {
    key_attributes: KeyAttributes,
    public_key: Vec<u8>,
}

impl CreateKeyChangeData {
    pub fn key_attributes(&self) -> &KeyAttributes {
        &self.key_attributes
    }
    pub fn public_key(&self) -> &[u8] {
        &self.public_key
    }
}

impl CreateKeyChangeData {
    pub fn new(key_attributes: KeyAttributes, public_key: Vec<u8>) -> Self {
        CreateKeyChangeData {
            key_attributes,
            public_key,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateKeyChange {
    data: CreateKeyChangeData,
    #[serde(with = "BigArray")]
    self_signature: [u8; 64],
}

impl CreateKeyChange {
    pub fn data(&self) -> &CreateKeyChangeData {
        &self.data
    }
    pub fn self_signature(&self) -> &[u8; 64] {
        &self.self_signature
    }
}

impl CreateKeyChange {
    pub fn new(data: CreateKeyChangeData, self_signature: [u8; 64]) -> Self {
        CreateKeyChange {
            data,
            self_signature,
        }
    }
}

impl Profile {
    pub(crate) fn create_key_event_static(
        prev_id: EventIdentifier,
        key_attributes: KeyAttributes,
        attributes: Option<ProfileEventAttributes>,
        root_key: Option<&Secret>,
        vault: &mut dyn ProfileVault,
    ) -> ockam_core::Result<ProfileChangeEvent> {
        let attributes = attributes.unwrap_or(ProfileEventAttributes::new());

        // TODO: Should be customisable
        let secret_attributes = SecretAttributes {
            stype: SecretType::Curve25519,
            persistence: SecretPersistence::Persistent,
            length: CURVE25519_SECRET_LENGTH,
        };

        let secret_key = vault.secret_generate(secret_attributes)?;
        let public_key = vault.secret_public_key_get(&secret_key)?;

        let data = CreateKeyChangeData::new(key_attributes, public_key.as_ref().to_vec());
        let data_binary = serde_bare::to_vec(&data).map_err(|_| OckamError::BareError)?;
        let data_hash = vault.sha256(data_binary.as_slice())?;
        let self_signature = vault.sign(&secret_key, &data_hash)?;
        let change = CreateKeyChange::new(data, self_signature);

        let profile_change = ProfileChange::new(
            PROFILE_CHANGE_CURRENT_VERSION,
            prev_id,
            attributes,
            ProfileChangeType::CreateKey(change),
        );
        let changes = vec![profile_change];
        let changes_binary = serde_bare::to_vec(&changes).map_err(|_| OckamError::BareError)?;

        let event_id = vault.sha256(&changes_binary)?;
        let event_id = EventIdentifier::from_hash(event_id);

        // If we have root_key passed we should sign using it
        // If there is no root_key - we're creating new profile, so we just generated root_key
        let sign_key;
        if let Some(root_key) = root_key {
            sign_key = root_key;
        } else {
            sign_key = &secret_key;
        }

        let signature = vault.sign(sign_key, event_id.as_ref())?;

        let proof =
            ProfileChangeProof::Signature(Signature::new(SignatureType::RootSign, signature));
        let signed_change_event = ProfileChangeEvent::new(event_id, changes, proof);

        Ok(signed_change_event)
    }

    pub(crate) fn create_key_event(
        &mut self,
        key_attributes: KeyAttributes,
        attributes: Option<ProfileEventAttributes>,
        root_key: Option<&Secret>,
    ) -> ockam_core::Result<ProfileChangeEvent> {
        // Creating key after it was revoked is forbidden
        if self
            .change_history
            .find_last_key_event(&key_attributes)
            .is_ok()
        {
            return Err(OckamError::InvalidInternalState.into());
        }

        let prev_id = self.change_history.get_last_event_id()?;

        Self::create_key_event_static(
            prev_id,
            key_attributes,
            attributes,
            root_key,
            self.vault.lock().unwrap().deref_mut(),
        )
    }
}
