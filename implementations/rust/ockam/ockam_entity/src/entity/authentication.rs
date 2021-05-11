//! Entity auth

use crate::EntityError::ProfileNotFound;
use crate::{Entity, ProfileAuth, ProfileIdentifier, ProfileTrait};
use ockam_core::Result;

impl<P: ProfileTrait> ProfileAuth for Entity<P> {
    fn generate_authentication_proof(&mut self, channel_state: &[u8]) -> Result<Vec<u8>> {
        for profile in &mut self.profiles {
            if self.default_profile_identifier == profile.identifier()? {
                return profile.generate_authentication_proof(channel_state);
            }
        }
        Err(ProfileNotFound.into())
    }

    fn verify_authentication_proof(
        &mut self,
        channel_state: &[u8],
        responder_contact_id: &ProfileIdentifier,
        proof: &[u8],
    ) -> Result<bool> {
        for profile in &mut self.profiles {
            if self.default_profile_identifier == profile.identifier()? {
                return profile.verify_authentication_proof(
                    channel_state,
                    responder_contact_id,
                    proof,
                );
            }
        }
        Err(ProfileNotFound.into())
    }
}
