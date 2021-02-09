use crate::ProfileChangeType::{CreateKey, RotateKey};
use crate::{
    EventIdentifier, KeyAttributes, OckamError, ProfileChange, ProfileChangeEvent, ProfileVault,
};
use ockam_vault_core::{PublicKey, Secret};

/// Full history of [`Profile`] changes. History and corresponding secret keys are enough to recreate [`Profile`]
#[derive(Clone, Debug)]
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

    pub(crate) fn get_secret_key_from_event(
        key_attributes: &KeyAttributes,
        event: &ProfileChangeEvent,
        vault: &dyn ProfileVault,
    ) -> ockam_core::Result<Secret> {
        let change = Self::find_key_change_in_event(event, key_attributes).unwrap(); // FIXME

        let public_key = Self::get_change_public_key(change).unwrap(); // FIXME

        let public_kid = vault.compute_key_id_for_public_key(&public_key)?;

        vault.get_secret_by_key_id(&public_kid)
    }
}
