use ockam_core::compat::vec::Vec;
use serde::{Deserialize, Serialize};

pub use crate::proof::*;
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
pub struct ChangeSet {
    prev_event_id: EventIdentifier,
    data: Vec<ProfileChange>,
}

impl ChangeSet {
    /// [`EventIdentifier`] of previous event
    pub fn previous_event_identifier(&self) -> &EventIdentifier {
        &self.prev_event_id
    }
    /// Set of changes been applied
    pub fn data(&self) -> &[ProfileChange] {
        &self.data
    }
}

impl ChangeSet {
    /// Create new Changes
    pub fn new(prev_event_id: EventIdentifier, data: Vec<ProfileChange>) -> Self {
        ChangeSet {
            prev_event_id,
            data,
        }
    }
}

/// [`crate::Profile`]s are modified using change events mechanism. One event may have 1 or more [`ProfileChange`]s
/// Proof is used to check whether this event comes from a party authorized to perform such updated
/// Individual changes may include additional proofs, if needed
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProfileChangeEvent {
    identifier: EventIdentifier,
    changes: ChangeSet,
    proof: ProfileChangeProof,
}

pub type Changes = Vec<ProfileChangeEvent>;

impl ProfileChangeEvent {
    /// Unique [`EventIdentifier`]
    pub fn identifier(&self) -> &EventIdentifier {
        &self.identifier
    }
    /// Set of changes been applied
    pub fn changes(&self) -> &ChangeSet {
        &self.changes
    }
    /// Proof is used to check whether this event comes from a party authorized to perform such updated
    /// Individual changes may include additional proofs, if needed
    pub fn proof(&self) -> &ProfileChangeProof {
        &self.proof
    }
}

impl ProfileChangeEvent {
    /// Create a new profile change event
    pub fn new(identifier: EventIdentifier, changes: ChangeSet, proof: ProfileChangeProof) -> Self {
        ProfileChangeEvent {
            identifier,
            changes,
            proof,
        }
    }
}
