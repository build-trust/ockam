mod identifier;
pub use identifier::*;

mod change;
pub use change::*;

mod verification;
pub use verification::*;

#[derive(Clone, Debug)]
pub struct Profile {
    identifier: ProfileIdentifier,
    change_history: ProfileChangeHistory,
    verification_policies: Vec<ProfileVerificationPolicy>,
}

impl Profile {
    pub fn identifier(&self) -> &ProfileIdentifier {
        &self.identifier
    }
    pub fn change_history(&self) -> &ProfileChangeHistory {
        &self.change_history
    }
    pub fn verification_policies(&self) -> &[ProfileVerificationPolicy] {
        &self.verification_policies
    }
}

impl Profile {
    pub fn new(
        identifier: ProfileIdentifier,
        change_history: ProfileChangeHistory,
        verification_policies: Vec<ProfileVerificationPolicy>,
    ) -> Self {
        Profile {
            identifier,
            change_history,
            verification_policies,
        }
    }
}

impl Profile {
    pub fn apply(&mut self, change_event: ProfileChangeEvent) {
        if !self.verify(&change_event) {
            return; // TODO: Throw error
        }

        for _change in change_event.changes() {
            // TODO: apply change
            unimplemented!()
        }
    }

    pub fn verify(&self, _change_event: &ProfileChangeEvent) -> bool {
        // loop over all proofs and verify them
        unimplemented!()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_new() {
        let id = ProfileIdentifier::from_hash([0u8; 32]);
        let _profile = Profile::new(id, ProfileChangeHistory::default(), Vec::new());
    }
}
