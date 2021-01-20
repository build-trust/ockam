use crate::profile::change_event::ChangeEventType::{CreateKey, RevokeKey, RotateKey};
use crate::profile::change_event::{Change, ProfileKeyPurpose, ProfileKeyType};
use crate::profile::error::Error;
use crate::profile::signed_change_event::SignedChangeEvent;
use crate::profile::{EventId, ProfileId, ProfileVault};
use ockam_common::error::OckamResult;
use ockam_vault::Secret;
use std::sync::{Arc, Mutex};

pub struct KeyEntry {
    event_id: EventId,
    key_type: ProfileKeyType,
    key_purpose: ProfileKeyPurpose,
    key: Arc<Mutex<Box<dyn Secret>>>,
}

impl KeyEntry {
    pub fn event_id(&self) -> &EventId {
        &self.event_id
    }
    pub fn key_type(&self) -> ProfileKeyType {
        self.key_type
    }
    pub fn key_purpose(&self) -> ProfileKeyPurpose {
        self.key_purpose
    }
    pub fn key(&self) -> &Arc<Mutex<Box<dyn Secret>>> {
        &self.key
    }
}

impl KeyEntry {
    pub fn new(
        event_id: EventId,
        key_type: ProfileKeyType,
        key_purpose: ProfileKeyPurpose,
        key: Arc<Mutex<Box<dyn Secret>>>,
    ) -> Self {
        KeyEntry {
            event_id,
            key_type,
            key_purpose,
            key,
        }
    }
}

pub struct Profile {
    identifier: ProfileId, // First public key id
    change_events: Vec<SignedChangeEvent>,
    keys: Vec<KeyEntry>,
    vault: Arc<Mutex<dyn ProfileVault>>,
}

impl Profile {
    pub fn identifier(&self) -> &ProfileId {
        &self.identifier
    }
    pub fn change_events(&self) -> &[SignedChangeEvent] {
        &self.change_events
    }
    pub fn keys(&self) -> &[KeyEntry] {
        &self.keys
    }
    pub fn vault(&self) -> &Arc<Mutex<dyn ProfileVault>> {
        &self.vault
    }
}

impl Profile {
    pub(crate) fn new(
        identifier: ProfileId,
        change_events: Vec<SignedChangeEvent>,
        keys: Vec<KeyEntry>,
        vault: Arc<Mutex<dyn ProfileVault>>,
    ) -> Self {
        Profile {
            identifier,
            change_events,
            keys,
            vault,
        }
    }

    pub(crate) fn get_last_event_id(&self) -> OckamResult<EventId> {
        if let Some(e) = self.change_events().last() {
            Ok(e.identifier().clone())
        } else {
            Err(Error::InvalidInternalState.into())
        }
    }

    pub(crate) fn get_private_key(
        &self,
        key_type: ProfileKeyType,
        key_purpose: ProfileKeyPurpose,
        event_id: &EventId,
    ) -> OckamResult<Arc<Mutex<Box<dyn Secret>>>> {
        self.keys()
            .iter()
            .rev()
            .find_map(|k| {
                if k.key_purpose() == key_purpose
                    && k.key_type() == key_type
                    && k.event_id() == event_id
                {
                    Some(k.key().clone())
                } else {
                    None
                }
            })
            .ok_or(Error::InvalidInternalState.into())
    }

    pub(crate) fn find_last_key_event(
        &self,
        key_type: ProfileKeyType,
        key_purpose: ProfileKeyPurpose,
    ) -> OckamResult<&SignedChangeEvent> {
        self.change_events
            .iter()
            .rev()
            .find(|e| {
                e.changes()
                    .as_ref()
                    .iter()
                    .rev()
                    .find(|c| match c.etype() {
                        CreateKey(event) => {
                            event.key_type() == key_type && event.key_purpose() == key_purpose
                        }
                        RotateKey(event) => {
                            event.key_type() == key_type && event.key_purpose() == key_purpose
                        }
                        RevokeKey(event) => {
                            event.key_type() == key_type && event.key_purpose() == key_purpose
                        }
                        _ => false,
                    })
                    .is_some()
            })
            .ok_or(Error::InvalidInternalState.into())
    }

    pub(crate) fn find_last_key_change(
        &self,
        key_type: ProfileKeyType,
        key_purpose: ProfileKeyPurpose,
    ) -> OckamResult<&Change> {
        self.change_events
            .iter()
            .rev()
            .find_map(|e| {
                e.changes().as_ref().iter().rev().find(|c| match c.etype() {
                    CreateKey(event) => {
                        event.key_type() == key_type && event.key_purpose() == key_purpose
                    }
                    RotateKey(event) => {
                        event.key_type() == key_type && event.key_purpose() == key_purpose
                    }
                    RevokeKey(event) => {
                        event.key_type() == key_type && event.key_purpose() == key_purpose
                    }
                    _ => false,
                })
            })
            .ok_or(Error::InvalidInternalState.into())
    }

    pub(crate) fn get_change_public_key(change: &Change) -> OckamResult<&[u8]> {
        match change.etype() {
            CreateKey(event) => Ok(event.public_key()),
            RotateKey(event) => Ok(event.public_key()),
            _ => Err(Error::InvalidInternalState.into()),
        }
    }

    pub(crate) fn public_key(
        &self,
        key_type: ProfileKeyType,
        key_purpose: ProfileKeyPurpose,
    ) -> OckamResult<&[u8]> {
        let last_change = self.find_last_key_change(key_type, key_purpose)?;

        Self::get_change_public_key(last_change)
    }

    pub(crate) fn add_event(
        &mut self,
        event: SignedChangeEvent,
        mut keys: Vec<KeyEntry>,
    ) -> OckamResult<()> {
        self.change_events.push(event);
        self.keys.append(&mut keys);

        Ok(())
    }
}
