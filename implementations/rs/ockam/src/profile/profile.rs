use crate::profile::error::Error;
use crate::profile::profile_event::ProfileEvent;
use crate::profile::ProfileVault;
use ockam_common::error::OckamResult;
use ockam_vault::Secret;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub type ProfileEventAttributes = HashMap<String, String>;

#[non_exhaustive]
pub struct ProfileEventAttributeKey;

impl ProfileEventAttributeKey {
    pub const FRIENDLY_NAME: &'static str = "OCKAM_FN";
    pub const CREATION_DATE: &'static str = "OCKAM_CD";
}

pub struct Profile {
    identifier: String,
    events: Vec<ProfileEvent>,
    vault: Arc<Mutex<dyn ProfileVault>>,
}

impl Profile {
    pub fn identifier(&self) -> &str {
        &self.identifier
    }
    pub fn events(&self) -> &[ProfileEvent] {
        &self.events
    }
    pub fn vault(&self) -> &Arc<Mutex<dyn ProfileVault>> {
        &self.vault
    }
}

impl Profile {
    pub(crate) fn new(
        identifier: String,
        events: Vec<ProfileEvent>,
        vault: Arc<Mutex<dyn ProfileVault>>,
    ) -> Self {
        Profile {
            identifier,
            events,
            vault,
        }
    }

    pub(crate) fn public_key(&self) -> OckamResult<Option<&[u8]>> {
        let event;
        if let Some(e) = self.events.last() {
            event = e;
        } else {
            return Err(Error::InvalidInternalState.into());
        }

        Ok(event.public_key())
    }

    pub(crate) fn rotate(&mut self, attributes: ProfileEventAttributes) -> OckamResult<()> {
        let event;
        if let Some(e) = self.events.last() {
            event = e;
        } else {
            return Err(Error::InvalidInternalState.into());
        }

        let new_event = ProfileEvent::new(false, attributes, Some(event), self.vault.clone())?;

        self.events.push(new_event);

        Ok(())
    }

    pub(crate) fn revoke(&mut self, attributes: ProfileEventAttributes) -> OckamResult<()> {
        let event;
        if let Some(e) = self.events.last() {
            event = e;
        } else {
            return Err(Error::InvalidInternalState.into());
        }

        let new_event = ProfileEvent::new(true, attributes, Some(event), self.vault.clone())?;

        self.events.push(new_event);

        Ok(())
    }

    pub(crate) fn attest(&self, nonce: &[u8]) -> OckamResult<[u8; 64]> {
        let event;
        if let Some(e) = self.events.last() {
            event = e;
        } else {
            return Err(Error::InvalidInternalState.into());
        }

        let private_key;
        if let Some(key) = event.private_key() {
            private_key = key;
        } else {
            return Err(Error::InvalidInternalState.into());
        }

        let mut vault = self.vault.lock().unwrap();

        Ok(vault.sign(private_key, nonce)?)
    }

    pub(crate) fn delete(&mut self) -> OckamResult<()> {
        let mut vault = self.vault.lock().unwrap();

        while let Some(mut event) = self.events.pop() {
            if let Some(private_key) = event.take_private_key() {
                vault.secret_destroy(private_key)?;
            }
        }

        Ok(())
    }
}
