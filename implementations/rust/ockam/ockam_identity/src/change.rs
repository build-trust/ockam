use crate::ChangeIdentifier;
use core::fmt;
use ockam_core::compat::vec::Vec;
use ockam_core::vault::PublicKey;
use ockam_core::Result;
use serde::{Deserialize, Serialize};

pub use crate::signature::*;

mod create_key;
mod rotate_key;

pub use create_key::*;
pub use rotate_key::*;

/// Possible types of [`crate::Identity`] changes
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum IdentityChange {
    /// Create key
    CreateKey(CreateKeyChangeData),
    /// Rotate key
    RotateKey(RotateKeyChangeData),
}

impl fmt::Display for IdentityChange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IdentityChange::CreateKey(data) => write!(f, " CreateKey:{}", data),
            IdentityChange::RotateKey(data) => write!(f, " RotateKey:{}", data),
        }
    }
}

impl IdentityChange {
    pub(crate) fn has_label(&self, label: &str) -> bool {
        self.label() == label
    }

    pub(crate) fn label(&self) -> &str {
        match self {
            IdentityChange::CreateKey(data) => data.key_attributes().label(),
            IdentityChange::RotateKey(data) => data.key_attributes().label(),
        }
    }

    pub(crate) fn public_key(&self) -> Result<PublicKey> {
        Ok(match self {
            IdentityChange::CreateKey(data) => data.public_key(),
            IdentityChange::RotateKey(data) => data.public_key(),
        }
        .clone())
    }

    pub(crate) fn previous_change_identifier(&self) -> &ChangeIdentifier {
        match self {
            IdentityChange::CreateKey(data) => data.prev_change_id(),
            IdentityChange::RotateKey(data) => data.prev_change_id(),
        }
    }
}

/// [`crate::Identity`]s are modified using a chain of changes.
/// Signatures are used to check change validity.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IdentitySignedChange {
    identifier: ChangeIdentifier,
    change: IdentityChange,
    signatures: Vec<IdentityChangeSignature>,
}

impl IdentitySignedChange {
    /// Unique [`ChangeIdentifier`]
    pub fn identifier(&self) -> &ChangeIdentifier {
        &self.identifier
    }
    /// Change been applied
    pub fn change(&self) -> &IdentityChange {
        &self.change
    }
    /// Signatures are used to check change validity.
    pub fn signatures(&self) -> &[IdentityChangeSignature] {
        &self.signatures
    }
}

impl IdentitySignedChange {
    /// Create a new identity change
    pub fn new(
        identifier: ChangeIdentifier,
        change: IdentityChange,
        signatures: Vec<IdentityChangeSignature>,
    ) -> Self {
        Self {
            identifier,
            change,
            signatures,
        }
    }
}

impl fmt::Display for IdentitySignedChange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "  identifier: {}", self.identifier())?;
        writeln!(f, "  identity change: {}", self.change())?;
        for s in self.signatures() {
            writeln!(f, "signatures: {}", s)?;
        }
        Ok(())
    }
}
