use crate::{EventIdentifier, ProfileChange, ProfileChangeProof};

pub type Changes = Vec<ProfileChange>;

/// [`Profile`]s are modified using change events mechanism. One event may have 1 or more [`ProfileChange`]s
/// Proof is used to check whether this event comes from a party authorized to perform such updated
/// Individual changes may include additional proofs, if needed
#[derive(Clone, Debug)]
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
