use crate::{EventIdentifier, ProfileChangeType, ProfileEventAttributes};
use serde::{Deserialize, Serialize};

/// Pre-defined keys in [`ProfileEventAttributes`] map
#[non_exhaustive]
pub struct ProfileEventAttributeKey;

impl ProfileEventAttributeKey {
    /// Human-readable name
    pub const FRIENDLY_NAME: &'static str = "OCKAM_FN";
    /// UTC timestamp
    pub const CREATION_DATE: &'static str = "OCKAM_CD";
}

/// Individual change applied to profile. [`ProfileChangeEvent`] consists of one or more such changes
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProfileChange {
    version: u8,
    prev_event_id: EventIdentifier,
    // TODO: Check attributes serialization
    attributes: ProfileEventAttributes,
    change_type: ProfileChangeType,
}

impl ProfileChange {
    /// Protocol version
    pub fn version(&self) -> u8 {
        self.version
    }
    /// [`EventIdentifier`] of previous event
    pub fn prev_event_id(&self) -> &EventIdentifier {
        &self.prev_event_id
    }
    /// User-specified attributes that will be saved with change
    pub fn attributes(&self) -> &ProfileEventAttributes {
        &self.attributes
    }
    /// Type of change along with type-specific data
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
