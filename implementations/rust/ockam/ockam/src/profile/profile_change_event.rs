use crate::{EventIdentifier, ProfileChange, ProfileChangeProof};

pub type Changes = Vec<ProfileChange>;

#[derive(Clone, Debug)]
pub struct ProfileChangeEvent {
    identifier: EventIdentifier,
    changes: Changes,
    proof: ProfileChangeProof,
}

impl ProfileChangeEvent {
    pub fn identifier(&self) -> &EventIdentifier {
        &self.identifier
    }
    pub fn changes(&self) -> &Changes {
        &self.changes
    }
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
