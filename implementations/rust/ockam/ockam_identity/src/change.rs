use crate::{EventIdentifier, IdentityEventAttributes};
use ockam_core::compat::vec::Vec;
use ockam_core::{vault::PublicKey, Result};
use serde::{Deserialize, Serialize};

pub use crate::signature::*;

mod create_key;
mod rotate_key;

pub use create_key::*;
pub use rotate_key::*;

/// Pre-defined keys in [`IdentityEventAttributes`] map
#[non_exhaustive]
pub struct IdentityEventAttributeKey;

impl IdentityEventAttributeKey {
    /// Human-readable name
    pub const FRIENDLY_NAME: &'static str = "OCKAM_FN";
    /// UTC timestamp
    pub const CREATION_DATE: &'static str = "OCKAM_CD";
}

/// Individual change applied to identity. [`IdentityChangeEvent`] consists of one or more such changes
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IdentityChange {
    version: u8,
    // TODO: Check attributes serialization
    attributes: IdentityEventAttributes,
    change_type: IdentityChangeType,
}

impl IdentityChange {
    /// Protocol version
    pub fn version(&self) -> u8 {
        self.version
    }
    /// User-specified attributes that will be saved with change
    pub fn attributes(&self) -> &IdentityEventAttributes {
        &self.attributes
    }
    /// Type of change along with type-specific data
    pub fn change_type(&self) -> &IdentityChangeType {
        &self.change_type
    }
}

impl IdentityChange {
    pub(crate) fn new(
        version: u8,
        attributes: IdentityEventAttributes,
        change_type: IdentityChangeType,
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
            IdentityChangeType::CreateKey(change) => change.data().key_attributes().label(),
            IdentityChangeType::RotateKey(change) => change.data().key_attributes().label(),
        }
    }

    pub(crate) fn public_key(&self) -> Result<PublicKey> {
        Ok(match &self.change_type {
            IdentityChangeType::CreateKey(change) => change.data().public_key(),
            IdentityChangeType::RotateKey(change) => change.data().public_key(),
        }
        .clone())
    }
}

/// Possible types of [`crate::Identity`] changes
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum IdentityChangeType {
    /// Create key
    CreateKey(CreateKeyChange),
    /// Rotate key
    RotateKey(RotateKeyChange),
}

/// Identity changes with a given event identifier
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeBlock {
    change: IdentityChange,
    prev_event_id: EventIdentifier,
}

impl ChangeBlock {
    /// [`EventIdentifier`] of previous event
    pub fn previous_event_identifier(&self) -> &EventIdentifier {
        &self.prev_event_id
    }
    /// Set of changes been applied
    pub fn change(&self) -> &IdentityChange {
        &self.change
    }
}

impl ChangeBlock {
    /// Create new Changes
    pub fn new(prev_event_id: EventIdentifier, change: IdentityChange) -> Self {
        Self {
            prev_event_id,
            change,
        }
    }
}

/// [`crate::Identity`]s are modified using change events mechanism. One event may have 1 or more [`IdentityChange`]s
/// Proof is used to check whether this event comes from a party authorized to perform such updated
/// Individual changes may include additional proofs, if needed
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IdentityChangeEvent {
    identifier: EventIdentifier,
    change_block: ChangeBlock,
    signatures: Vec<Signature>,
}

pub type Changes = Vec<IdentityChangeEvent>;

impl IdentityChangeEvent {
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

impl IdentityChangeEvent {
    /// Create a new identity change event
    pub fn new(
        identifier: EventIdentifier,
        change_block: ChangeBlock,
        signatures: Vec<Signature>,
    ) -> Self {
        IdentityChangeEvent {
            identifier,
            change_block,
            signatures,
        }
    }
}
