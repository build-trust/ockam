//! Entity secrets

use crate::EntityError::ProfileNotFound;
use crate::{Entity, KeyAttributes, ProfileEventAttributes, ProfileSecrets, ProfileTrait};
use ockam_core::Result;
use ockam_vault_core::{PublicKey, Secret};

impl<P: ProfileTrait> ProfileSecrets for Entity<P> {
    fn create_key(
        &mut self,
        key_attributes: KeyAttributes,
        attributes: Option<ProfileEventAttributes>,
    ) -> Result<()> {
        if let Some(profile) = self.profiles.get_mut(&self.default_profile_identifier) {
            profile.create_key(key_attributes, attributes)
        } else {
            Err(ProfileNotFound.into())
        }
    }

    fn rotate_key(
        &mut self,
        key_attributes: KeyAttributes,
        attributes: Option<ProfileEventAttributes>,
    ) -> Result<()> {
        if let Some(profile) = self.profiles.get_mut(&self.default_profile_identifier) {
            profile.rotate_key(key_attributes, attributes)
        } else {
            Err(ProfileNotFound.into())
        }
    }

    fn get_secret_key(&mut self, key_attributes: &KeyAttributes) -> Result<Secret> {
        if let Some(profile) = self.profiles.get_mut(&self.default_profile_identifier) {
            profile.get_secret_key(key_attributes)
        } else {
            Err(ProfileNotFound.into())
        }
    }

    fn get_public_key(&self, key_attributes: &KeyAttributes) -> Result<PublicKey> {
        if let Some(profile) = self.default_profile() {
            profile.get_public_key(key_attributes)
        } else {
            Err(ProfileNotFound.into())
        }
    }

    fn get_root_secret(&mut self) -> Result<Secret> {
        if let Some(profile) = self.profiles.get_mut(&self.default_profile_identifier) {
            profile.get_root_secret()
        } else {
            Err(ProfileNotFound.into())
        }
    }
}
