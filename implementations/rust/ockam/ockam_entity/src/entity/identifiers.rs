//! Entity identifiers

use crate::{Entity, ProfileIdentifier, ProfileIdentity, ProfileTrait};
use ockam_core::Result;

impl<P: ProfileTrait> ProfileIdentity for Entity<P> {
    fn identifier(&self) -> Result<ProfileIdentifier> {
        Ok(self.default_profile_identifier.clone())
    }
}
