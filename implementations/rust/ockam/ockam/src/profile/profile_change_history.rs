use crate::ProfileChangeType::{CreateKey, RotateKey};
use crate::{
    EventIdentifier, KeyAttributes, OckamError, ProfileChange, ProfileChangeEvent,
    ProfileChangeProof, ProfileChangeType, ProfileVault, SignatureType,
};
use ockam_vault_core::PublicKey;
use serde::{Deserialize, Serialize};

/// Full history of [`Profile`] changes. History and corresponding secret keys are enough to recreate [`Profile`]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct ProfileChangeHistory(Vec<ProfileChangeEvent>);

impl ProfileChangeHistory {
    pub(crate) fn new(change_events: Vec<ProfileChangeEvent>) -> Self {
        Self(change_events)
    }

    pub(crate) fn push_event(&mut self, event: ProfileChangeEvent) {
        self.0.push(event)
    }
}

impl AsRef<[ProfileChangeEvent]> for ProfileChangeHistory {
    fn as_ref(&self) -> &[ProfileChangeEvent] {
        &self.0
    }
}

impl Default for ProfileChangeHistory {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

impl ProfileChangeHistory {
    pub(crate) fn get_last_event_id(&self) -> ockam_core::Result<EventIdentifier> {
        if let Some(e) = self.0.last() {
            Ok(e.identifier().clone())
        } else {
            Err(OckamError::InvalidInternalState.into())
        }
    }

    pub(crate) fn find_key_change_in_event<'a>(
        event: &'a ProfileChangeEvent,
        key_attributes: &KeyAttributes,
    ) -> Option<&'a ProfileChange> {
        event
            .changes()
            .data()
            .iter()
            .rev()
            .find(|c| match c.change_type() {
                CreateKey(change) => change.data().key_attributes() == key_attributes,
                RotateKey(change) => change.data().key_attributes() == key_attributes, // RevokeKey(event) => {
                                                                                       //     event.key_type() == key_type && event.key_purpose() == key_purpose && event.label() == label
                                                                                       // }
            })
    }

    pub(crate) fn find_last_key_event(
        &self,
        key_attributes: &KeyAttributes,
    ) -> ockam_core::Result<&ProfileChangeEvent> {
        self.0
            .iter()
            .rev()
            .find(|e| Self::find_key_change_in_event(e, key_attributes).is_some())
            .ok_or(OckamError::InvalidInternalState.into())
    }

    pub(crate) fn find_key_event_before(
        &self,
        event_id: &EventIdentifier,
        key_attributes: &KeyAttributes,
    ) -> ockam_core::Result<&ProfileChangeEvent> {
        let before_index = self
            .0
            .iter()
            .position(|e| e.identifier() == event_id)
            .unwrap_or(self.0.len());
        self.0[..before_index]
            .iter()
            .rev()
            .find(|e| Self::find_key_change_in_event(e, key_attributes).is_some())
            .ok_or(OckamError::InvalidInternalState.into())
    }

    pub(crate) fn get_change_public_key(change: &ProfileChange) -> ockam_core::Result<PublicKey> {
        let data = match change.change_type() {
            CreateKey(change) => change.data().public_key(),
            RotateKey(change) => change.data().public_key(),
        };

        Ok(PublicKey::new(data.into()))
    }

    pub(crate) fn get_public_key_from_event(
        key_attributes: &KeyAttributes,
        event: &ProfileChangeEvent,
    ) -> ockam_core::Result<PublicKey> {
        let change = Self::find_key_change_in_event(event, key_attributes)
            .ok_or_else(|| OckamError::InvalidInternalState)?;

        Self::get_change_public_key(change)
    }
}

impl ProfileChangeHistory {
    pub(crate) fn get_root_public_key(&self) -> ockam_core::Result<PublicKey> {
        let root_event;
        if let Some(re) = self.as_ref().first() {
            root_event = re;
        } else {
            return Err(OckamError::InvalidInternalState.into());
        }

        let root_change;
        if let Some(rc) = root_event.changes().data().first() {
            root_change = rc;
        } else {
            return Err(OckamError::InvalidInternalState.into());
        }

        let root_create_key_change;
        if let ProfileChangeType::CreateKey(c) = root_change.change_type() {
            root_create_key_change = c;
        } else {
            return Err(OckamError::InvalidInternalState.into());
        }

        Ok(PublicKey::new(
            root_create_key_change.data().public_key().to_vec().into(),
        ))
    }

    pub(crate) fn get_public_key(
        &self,
        key_attributes: &KeyAttributes,
    ) -> ockam_core::Result<PublicKey> {
        let event = self.find_last_key_event(key_attributes)?;
        Self::get_public_key_from_event(key_attributes, event)
    }
}

impl ProfileChangeHistory {
    /// WARNING: This function assumes all existing events in chain are verified.
    /// WARNING: Correctness of events sequence is not verified here.
    pub(crate) fn verify(
        &self,
        change_event: &ProfileChangeEvent,
        vault: &mut dyn ProfileVault,
    ) -> ockam_core::Result<()> {
        let changes = change_event.changes();
        let changes_binary = serde_bare::to_vec(&changes).map_err(|_| OckamError::BareError)?;

        let event_id = vault.sha256(&changes_binary)?;
        let event_id = EventIdentifier::from_hash(event_id);

        if &event_id != change_event.identifier() {
            return Err(OckamError::EventIdDoesntMatch.into());
        }

        match change_event.proof() {
            ProfileChangeProof::Signature(s) => match s.stype() {
                SignatureType::RootSign => {
                    let root_public_key = self.get_root_public_key()?;
                    vault.verify(s.data(), root_public_key.as_ref(), event_id.as_ref())?;
                }
            },
        }

        for change in change_event.changes().data() {
            if !match change.change_type() {
                CreateKey(c) => {
                    // Should have 1 self signature
                    let data_binary =
                        serde_bare::to_vec(c.data()).map_err(|_| OckamError::BareError)?;
                    let data_hash = vault.sha256(data_binary.as_slice())?;

                    vault
                        .verify(c.self_signature(), c.data().public_key(), &data_hash)
                        .is_ok()
                }
                RotateKey(c) => {
                    // Should have 1 self signature and 1 prev signature
                    let data_binary =
                        serde_bare::to_vec(c.data()).map_err(|_| OckamError::BareError)?;
                    let data_hash = vault.sha256(data_binary.as_slice())?;

                    if !vault
                        .verify(c.self_signature(), c.data().public_key(), &data_hash)
                        .is_ok()
                    {
                        false;
                    }

                    let prev_key_event =
                        self.find_key_event_before(&event_id, c.data().key_attributes())?;
                    let prev_key_change = ProfileChangeHistory::find_key_change_in_event(
                        prev_key_event,
                        c.data().key_attributes(),
                    )
                    .ok_or_else(|| OckamError::InvalidInternalState)?;
                    let public_key = ProfileChangeHistory::get_change_public_key(prev_key_change)?;

                    vault
                        .verify(c.prev_signature(), public_key.as_ref(), &data_hash)
                        .is_ok()
                }
            } {
                return Err(OckamError::VerifyFailed.into());
            }
        }

        Ok(())
    }
}
