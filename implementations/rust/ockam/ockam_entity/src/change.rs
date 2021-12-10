use ockam_core::compat::vec::Vec;
use ockam_core::vault::PublicKey;
use ockam_core::Result;
use serde::{Deserialize, Serialize};

pub use crate::signature::*;
use crate::{CreateKeyChange, EventIdentifier, ProfileEventAttributes, RotateKeyChange};

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
    // TODO: Check attributes serialization
    attributes: ProfileEventAttributes,
    change_type: ProfileChangeType,
}

impl ProfileChange {
    /// Protocol version
    pub fn version(&self) -> u8 {
        self.version
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
        attributes: ProfileEventAttributes,
        change_type: ProfileChangeType,
    ) -> Self {
        Self {
            version,
            attributes,
            change_type,
        }
    }

    pub fn has_label(&self, label: &str) -> bool {
        self.label() == label
    }

    pub fn label(&self) -> &str {
        match &self.change_type {
            ProfileChangeType::CreateKey(change) => change.data().key_attributes().label(),
            ProfileChangeType::RotateKey(change) => change.data().key_attributes().label(),
        }
    }

    pub(crate) fn public_key(&self) -> Result<PublicKey> {
        Ok(match &self.change_type {
            ProfileChangeType::CreateKey(change) => change.data().public_key(),
            ProfileChangeType::RotateKey(change) => change.data().public_key(),
        }
        .clone())
    }
}

/// Possible types of [`crate::Profile`] changes
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ProfileChangeType {
    /// Create key
    CreateKey(CreateKeyChange),
    /// Rotate key
    RotateKey(RotateKeyChange),
}

/// Profile changes with a given event identifier
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeBlock {
    change: ProfileChange,
    prev_event_id: EventIdentifier,
}

impl ChangeBlock {
    /// [`EventIdentifier`] of previous event
    pub fn previous_event_identifier(&self) -> &EventIdentifier {
        &self.prev_event_id
    }
    /// Set of changes been applied
    pub fn change(&self) -> &ProfileChange {
        &self.change
    }
}

impl ChangeBlock {
    /// Create new Changes
    pub fn new(prev_event_id: EventIdentifier, change: ProfileChange) -> Self {
        Self {
            prev_event_id,
            change,
        }
    }
}

/// [`crate::Profile`]s are modified using change events mechanism. One event may have 1 or more [`ProfileChange`]s
/// Proof is used to check whether this event comes from a party authorized to perform such updated
/// Individual changes may include additional proofs, if needed
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProfileChangeEvent {
    identifier: EventIdentifier,
    change_block: ChangeBlock,
    signatures: Vec<Signature>,
}

pub type Changes = Vec<ProfileChangeEvent>;

impl ProfileChangeEvent {
    /// Unique [`EventIdentifier`]
    pub fn identifier(&self) -> &EventIdentifier {
        &self.identifier
    }
    /// Set of changes been applied
    pub fn change_block(&self) -> &ChangeBlock {
        &self.change_block
    }
    /// Proof is used to check whether this event comes from a party authorized to perform such update
    pub fn signatures(&self) -> &[Signature] {
        &self.signatures
    }
}

impl ProfileChangeEvent {
    /// Create a new profile change event
    pub fn new(
        identifier: EventIdentifier,
        change_block: ChangeBlock,
        signatures: Vec<Signature>,
    ) -> Self {
        ProfileChangeEvent {
            identifier,
            change_block,
            signatures,
        }
    }
}
