//! Entity auth

use crate::EntityError::ProfileNotFound;
use crate::{Entity, ProfileAuth, ProfileIdentifier, ProfileTrait};
use ockam_core::Result;

impl<P: ProfileTrait> ProfileAuth for Entity<P> {
    fn generate_authentication_proof(&mut self, channel_state: &[u8]) -> Result<Vec<u8>> {
        if let Some(profile) = self.profiles.get_mut(&self.default_profile_identifier) {
            profile.generate_authentication_proof(channel_state)
        } else {
            Err(ProfileNotFound.into())
        }
    }

    fn verify_authentication_proof(
        &mut self,
        channel_state: &[u8],
        responder_contact_id: &ProfileIdentifier,
        proof: &[u8],
    ) -> Result<bool> {
        if let Some(profile) = self.profiles.get_mut(&self.default_profile_identifier) {
            profile.verify_authentication_proof(channel_state, responder_contact_id, proof)
        } else {
            Err(ProfileNotFound.into())
        }
    }
}
