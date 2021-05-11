//! Entity changes

use crate::EntityError::ProfileNotFound;
use crate::{Entity, ProfileChangeEvent, ProfileChanges, ProfileTrait};
use ockam_core::Result;

impl<P: ProfileTrait> ProfileChanges for Entity<P> {
    fn change_events(&self) -> Result<Vec<ProfileChangeEvent>> {
        if let Some(profile) = self.default_profile() {
            profile.change_events()
        } else {
            Err(ProfileNotFound.into())
        }
    }

    fn update_no_verification(&mut self, change_event: ProfileChangeEvent) -> Result<()> {
        for profile in &mut self.profiles {
            if self.default_profile_identifier == profile.identifier()? {
                return profile.update_no_verification(change_event);
            }
        }
        Err(ProfileNotFound.into())
    }

    fn verify(&mut self) -> Result<bool> {
        for profile in &mut self.profiles {
            if self.default_profile_identifier == profile.identifier()? {
                return profile.verify();
            }
        }
        Err(ProfileNotFound.into())
    }
}
