//! Entity identifiers

use crate::{ProfileIdentifier, ProfileIdentity, ProfileSet, ProfileTrait};
use ockam_core::Result;

impl<P: ProfileTrait> ProfileIdentity for ProfileSet<P> {
    fn identifier(&self) -> Result<ProfileIdentifier> {
        Ok(self.default_profile_identifier.clone())
    }
}
