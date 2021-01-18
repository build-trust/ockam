use super::*;

#[derive(Clone, Debug)]
pub struct ProfileChangeIdentifier(ProfileIdentifier);

impl ProfileChangeIdentifier {
    fn apply(&self, profile: &mut Profile) {
        profile.identifier = self.0.clone();
    }
}

// Variants of changes allowed in a change event.
#[derive(Clone, Debug)]
pub enum ProfileChange {
    Identifier(ProfileChangeIdentifier),
}

impl ProfileChange {
    fn apply(&self, profile: &mut Profile) {
        match self {
            ProfileChange::Identifier(change) => change.apply(profile),
        }
    }
}

// Variants of proofs that are allowed on a change event.
#[derive(Clone, Debug)]
pub enum ProfileChangeProof {}

#[derive(Clone, Debug)]
pub struct ProfileChangeEvent {
    pub changes: Vec<ProfileChange>,
    pub proofs: Vec<ProfileChangeProof>,
}

impl ProfileChangeEvent {
    pub fn new(changes: &[ProfileChange], proofs: &[ProfileChangeProof]) -> Self {
        ProfileChangeEvent {
            changes: changes.to_vec(),
            proofs: proofs.to_vec(),
        }
    }

    pub fn verify(&self, _profile: &mut Profile) -> bool {
        // loop over all proofs and verify them
        true
    }

    pub fn apply(&self, profile: &mut Profile) {
        let verified = self.verify(profile);
        if verified {
            for change in &self.changes {
                change.apply(profile)
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct ProfileChangeHistory(Vec<ProfileChangeEvent>);

impl ProfileChangeHistory {
    pub fn new() -> Self {
        ProfileChangeHistory(vec![])
    }
}

impl Default for ProfileChangeHistory {
    fn default() -> Self {
        Self::new()
    }
}
