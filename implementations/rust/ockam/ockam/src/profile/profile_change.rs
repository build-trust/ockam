use crate::{EventIdentifier, ProfileChangeType, ProfileEventAttributes};
use serde::{Deserialize, Serialize};

#[non_exhaustive]
pub struct ProfileEventAttributeKey;

impl ProfileEventAttributeKey {
    pub const FRIENDLY_NAME: &'static str = "OCKAM_FN";
    pub const CREATION_DATE: &'static str = "OCKAM_CD";
}

// Variants of changes allowed in a change event.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProfileChange {
    version: u8,
    prev_event_id: EventIdentifier,
    // TODO: Check attributes serialization
    attributes: ProfileEventAttributes,
    change_type: ProfileChangeType,
}

impl ProfileChange {
    pub fn version(&self) -> u8 {
        self.version
    }
    pub fn prev_event_id(&self) -> &EventIdentifier {
        &self.prev_event_id
    }
    pub fn attributes(&self) -> &ProfileEventAttributes {
        &self.attributes
    }
    pub fn change_type(&self) -> &ProfileChangeType {
        &self.change_type
    }
}

impl ProfileChange {
    pub(crate) fn new(
        version: u8,
        prev_event_id: EventIdentifier,
        attributes: ProfileEventAttributes,
        change_type: ProfileChangeType,
    ) -> Self {
        Self {
            version,
            prev_event_id,
            attributes,
            change_type,
        }
    }
}
