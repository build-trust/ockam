use crate::profile::profile::ProfileEventAttributes;
use crate::profile::profile_event::ProfileEvent;
use ockam_common::error::OckamResult;

#[derive(Clone)]
pub struct ContactEvent {
    version: u8,
    identifier: String,
    model_binary: Vec<u8>,
    // TODO: Check attributes serialization
    attributes: ProfileEventAttributes,
    public_key: Option<Vec<u8>>,
    prev_event_id: Option<String>,
    next_event_id: Option<String>,
    self_signature: Option<[u8; 64]>,
    previous_self_signature: Option<[u8; 64]>,
}

impl ContactEvent {
    pub fn version(&self) -> u8 {
        self.version
    }
    pub fn identifier(&self) -> &str {
        &self.identifier
    }
    pub fn model_binary(&self) -> &Vec<u8> {
        &self.model_binary
    }
    pub fn attributes(&self) -> &ProfileEventAttributes {
        &self.attributes
    }
    pub fn public_key(&self) -> &Option<Vec<u8>> {
        &self.public_key
    }
    pub fn prev_event_id(&self) -> &Option<String> {
        &self.prev_event_id
    }
    pub fn next_event_id(&self) -> &Option<String> {
        &self.next_event_id
    }
    pub fn self_signature(&self) -> Option<[u8; 64]> {
        self.self_signature
    }
    pub fn previous_self_signature(&self) -> Option<[u8; 64]> {
        self.previous_self_signature
    }
}

impl ContactEvent {
    pub fn from_profile_event(profile_event: &ProfileEvent) -> OckamResult<Self> {
        Ok(Self {
            version: profile_event.version(),
            identifier: profile_event.identifier().to_string(),
            model_binary: profile_event.model_binary().clone(),
            attributes: profile_event.attributes().clone(),
            public_key: profile_event.public_key().clone(),
            prev_event_id: profile_event.prev_event_id().clone(),
            next_event_id: profile_event.next_event_id().clone(),
            self_signature: profile_event.self_signature().clone(),
            previous_self_signature: profile_event.previous_self_signature().clone(),
        })
    }
}
