use crate::profile::change_event::ChangeEventType::{CreateKey, RevokeKey, RotateKey};
use crate::profile::change_event::{ProfileKeyPurpose, ProfileKeyType};
use crate::profile::error::Error;
use crate::profile::signed_change_event::SignedChangeEvent;
use crate::profile::{EventId, ProfileId, ProfileVault};
use ockam_common::error::OckamResult;
use ockam_vault::Secret;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct Profile {
    identifier: ProfileId, // First public key id
    change_events: Vec<SignedChangeEvent>,
    keys: HashMap<EventId, Arc<Mutex<Box<dyn Secret>>>>,
    vault: Arc<Mutex<dyn ProfileVault>>,
}

impl Profile {
    pub fn identifier(&self) -> &ProfileId {
        &self.identifier
    }
    pub fn change_events(&self) -> &[SignedChangeEvent] {
        &self.change_events
    }
    pub fn keys(&self) -> &HashMap<EventId, Arc<Mutex<Box<dyn Secret>>>> {
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
        keys: HashMap<EventId, Arc<Mutex<Box<dyn Secret>>>>,
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
        event_id: &EventId,
    ) -> OckamResult<Arc<Mutex<Box<dyn Secret>>>> {
        if let Some(k) = self.keys().get(event_id) {
            Ok(k.clone())
        } else {
            Err(Error::InvalidInternalState.into())
        }
    }

    pub(crate) fn find_last_key_event(
        &self,
        key_type: ProfileKeyType,
        key_purpose: ProfileKeyPurpose,
    ) -> OckamResult<&SignedChangeEvent> {
        self.change_events
            .iter()
            .rev()
            .find(|e| match e.change_event().etype() {
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
            .ok_or(Error::InvalidInternalState.into())
    }

    pub(crate) fn get_event_public_key(event: &SignedChangeEvent) -> OckamResult<&[u8]> {
        match event.change_event().etype() {
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
        let last_event = self.find_last_key_event(key_type, key_purpose)?;

        Self::get_event_public_key(last_event)
    }

    pub(crate) fn add_event(
        &mut self,
        event: SignedChangeEvent,
        key: Option<Arc<Mutex<Box<dyn Secret>>>>,
    ) -> OckamResult<()> {
        let event_id = event.identifier().clone();
        self.change_events.push(event);
        if let Some(key) = key {
            self.keys.insert(event_id, key);
        }

        Ok(())
    }
}
