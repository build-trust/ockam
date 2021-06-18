use crate::history::ProfileChangeHistory;
use crate::{
    Changes, EntityError, EventIdentifier, KeyAttributes, Profile, ProfileChange,
    ProfileChangeEvent, ProfileChangeProof, ProfileChangeType, ProfileEventAttributes, ProfileImpl,
    ProfileVault, Signature, SignatureType,
};
use ockam_vault_core::{
    Secret, SecretAttributes, SecretPersistence, SecretType, CURVE25519_SECRET_LENGTH,
};
use serde::{Deserialize, Serialize};
use serde_big_array::big_array;

big_array! { BigArray; }

/// Key change data creation
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateKeyChangeData {
    key_attributes: KeyAttributes,
    public_key: Vec<u8>,
}

impl CreateKeyChangeData {
    /// Return key attributes
    pub fn key_attributes(&self) -> &KeyAttributes {
        &self.key_attributes
    }
    /// Return public key
    pub fn public_key(&self) -> &[u8] {
        &self.public_key
    }
}

impl CreateKeyChangeData {
    /// Create new CreateKeyChangeData
    pub fn new(key_attributes: KeyAttributes, public_key: Vec<u8>) -> Self {
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
    #[serde(with = "BigArray")]
    self_signature: [u8; 64],
}

impl CreateKeyChange {
    /// Return data
    pub fn data(&self) -> &CreateKeyChangeData {
        &self.data
    }
    /// Return self signature
    pub fn self_signature(&self) -> &[u8; 64] {
        &self.self_signature
    }
}

impl CreateKeyChange {
    /// Create new CreateKeyChange
    pub fn new(data: CreateKeyChangeData, self_signature: [u8; 64]) -> Self {
        CreateKeyChange {
            data,
            self_signature,
        }
    }
}

impl<V: ProfileVault> ProfileImpl<V> {
    /// Create a new key event
    pub(crate) fn create_key_event_static(
        prev_id: EventIdentifier,
        key_attributes: KeyAttributes,
        attributes: Option<ProfileEventAttributes>,
        root_key: Option<&Secret>,
        vault: &mut V,
    ) -> ockam_core::Result<ProfileChangeEvent> {
        let attributes = attributes.unwrap_or_default();

        // FIXME
        let is_bls = key_attributes.label() == Profile::SIGNING;

        let secret_attributes = if is_bls {
            SecretAttributes::new(SecretType::Bls, SecretPersistence::Persistent, 32)
        } else {
            SecretAttributes::new(
                SecretType::Curve25519,
                SecretPersistence::Persistent,
                CURVE25519_SECRET_LENGTH,
            )
        };

        let secret_key = vault.secret_generate(secret_attributes)?;
        let public_key = vault.secret_public_key_get(&secret_key)?;

        let data = CreateKeyChangeData::new(key_attributes, public_key.as_ref().to_vec());
        let data_binary = serde_bare::to_vec(&data).map_err(|_| EntityError::BareError)?;
        let data_hash = vault.sha256(data_binary.as_slice())?;
        let self_signature = if is_bls {
            [0u8; 64]
        } else {
            vault.sign(&secret_key, &data_hash)?
        };
        let change = CreateKeyChange::new(data, self_signature);

        let profile_change = ProfileChange::new(
            Profile::CURRENT_CHANGE_VERSION,
            attributes,
            ProfileChangeType::CreateKey(change),
        );
        let changes = Changes::new(prev_id, vec![profile_change]);
        let changes_binary = serde_bare::to_vec(&changes).map_err(|_| EntityError::BareError)?;

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

    /// Create a new key event
    pub(crate) fn create_key_event(
        &mut self,
        key_attributes: KeyAttributes,
        attributes: Option<ProfileEventAttributes>,
        root_key: Option<&Secret>,
    ) -> ockam_core::Result<ProfileChangeEvent> {
        // Creating key after it was revoked is forbidden
        if ProfileChangeHistory::find_last_key_event(
            self.change_history().as_ref(),
            &key_attributes,
        )
        .is_ok()
        {
            return Err(EntityError::InvalidInternalState.into());
        }

        let prev_id = self.change_history().get_last_event_id()?;

        Self::create_key_event_static(
            prev_id,
            key_attributes,
            attributes,
            root_key,
            &mut self.vault,
        )
    }
}
