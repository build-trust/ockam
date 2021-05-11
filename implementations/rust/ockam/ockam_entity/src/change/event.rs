use crate::{EventIdentifier, ProfileChange, ProfileChangeProof};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Changes {
    prev_event_id: EventIdentifier,
    data: Vec<ProfileChange>,
}

impl Changes {
    /// [`EventIdentifier`] of previous event
    pub fn previous_event_identifier(&self) -> &EventIdentifier {
        &self.prev_event_id
    }
    /// Set of changes been applied
    pub fn data(&self) -> &[ProfileChange] {
        &self.data
    }
}

impl Changes {
    pub fn new(prev_event_id: EventIdentifier, data: Vec<ProfileChange>) -> Self {
        Changes {
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
    changes: Changes,
    proof: ProfileChangeProof,
}

impl ProfileChangeEvent {
    /// Unique [`EventIdentifier`]
    pub fn identifier(&self) -> &EventIdentifier {
        &self.identifier
    }
    /// Set of changes been applied
    pub fn changes(&self) -> &Changes {
        &self.changes
    }
    /// Proof is used to check whether this event comes from a party authorized to perform such updated
    /// Individual changes may include additional proofs, if needed
    pub fn proof(&self) -> &ProfileChangeProof {
        &self.proof
    }
}

impl ProfileChangeEvent {
    pub fn new(identifier: EventIdentifier, changes: Changes, proof: ProfileChangeProof) -> Self {
        ProfileChangeEvent {
            identifier,
            changes,
            proof,
        }
    }
}
