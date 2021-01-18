use crate::profile::{EventId, ProfileEventAdditionalData, ProfileEventAttributes};
use serde::{Deserialize, Serialize};

#[non_exhaustive]
pub struct ProfileEventAttributeKey;

impl ProfileEventAttributeKey {
    pub const FRIENDLY_NAME: &'static str = "OCKAM_FN";
    pub const CREATION_DATE: &'static str = "OCKAM_CD";
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Eq, PartialEq)]
pub enum ProfileKeyType {
    Main,
    Additional,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Eq, PartialEq)]
pub enum ProfileKeyPurpose {
    Kex,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChangeEvent {
    version: u8,
    prev_event_id: EventId,
    // TODO: Check attributes serialization
    attributes: ProfileEventAttributes,
    etype: ChangeEventType,
}

impl ChangeEvent {
    pub fn version(&self) -> u8 {
        self.version
    }
    pub fn prev_event_id(&self) -> &EventId {
        &self.prev_event_id
    }
    pub fn attributes(&self) -> &ProfileEventAttributes {
        &self.attributes
    }
    pub fn etype(&self) -> &ChangeEventType {
        &self.etype
    }
}

impl ChangeEvent {
    pub(crate) fn new(
        version: u8,
        prev_event_id: EventId,
        attributes: ProfileEventAttributes,
        etype: ChangeEventType,
    ) -> Self {
        ChangeEvent {
            version,
            prev_event_id,
            attributes,
            etype,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ChangeEventType {
    CreateKey(CreateKeyEvent),
    RotateKey(RotateKeyEvent),
    RevokeKey(RevokeKeyEvent),
    ChangeAttributes(ChangeAdditionalDataEvent),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateKeyEvent {
    key_type: ProfileKeyType,
    key_purpose: ProfileKeyPurpose,
    public_key: Vec<u8>,
}

impl CreateKeyEvent {
    pub fn key_type(&self) -> ProfileKeyType {
        self.key_type
    }
    pub fn key_purpose(&self) -> ProfileKeyPurpose {
        self.key_purpose
    }
    pub fn public_key(&self) -> &[u8] {
        &self.public_key
    }
}

impl CreateKeyEvent {
    pub(crate) fn new(
        key_type: ProfileKeyType,
        key_purpose: ProfileKeyPurpose,
        public_key: Vec<u8>,
    ) -> Self {
        CreateKeyEvent {
            key_type,
            key_purpose,
            public_key,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RotateKeyEvent {
    key_type: ProfileKeyType,
    key_purpose: ProfileKeyPurpose,
    public_key: Vec<u8>,
}

impl RotateKeyEvent {
    pub fn key_type(&self) -> ProfileKeyType {
        self.key_type
    }
    pub fn key_purpose(&self) -> ProfileKeyPurpose {
        self.key_purpose
    }
    pub fn public_key(&self) -> &[u8] {
        &self.public_key
    }
}

impl RotateKeyEvent {
    pub(crate) fn new(
        key_type: ProfileKeyType,
        key_purpose: ProfileKeyPurpose,
        public_key: Vec<u8>,
    ) -> Self {
        RotateKeyEvent {
            key_type,
            key_purpose,
            public_key,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RevokeKeyEvent {
    key_type: ProfileKeyType,
    key_purpose: ProfileKeyPurpose,
}

impl RevokeKeyEvent {
    pub fn key_type(&self) -> ProfileKeyType {
        self.key_type
    }
    pub fn key_purpose(&self) -> ProfileKeyPurpose {
        self.key_purpose
    }
}

impl RevokeKeyEvent {
    pub fn new(key_type: ProfileKeyType, key_purpose: ProfileKeyPurpose) -> Self {
        RevokeKeyEvent {
            key_type,
            key_purpose,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChangeAdditionalDataEvent {
    set: ProfileEventAdditionalData,
    delete: Vec<String>,
}

impl ChangeAdditionalDataEvent {
    pub fn set(&self) -> &ProfileEventAdditionalData {
        &self.set
    }
    pub fn delete(&self) -> &[String] {
        &self.delete
    }
}

impl ChangeAdditionalDataEvent {
    pub(crate) fn new(set: ProfileEventAdditionalData, delete: Vec<String>) -> Self {
        ChangeAdditionalDataEvent { set, delete }
    }
}
