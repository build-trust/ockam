use crate::profile::error::Error;
use crate::profile::profile::{Profile, ProfileEventAttributes};
use crate::profile::profile_event::ProfileEvent;
use crate::profile::ProfileVault;
use ockam_common::error::OckamResult;
use std::sync::{Arc, Mutex};

pub struct ProfileManager {}

impl ProfileManager {
    pub fn new() -> Self {
        ProfileManager {}
    }

    pub fn create_profile(
        &self,
        attributes: Option<ProfileEventAttributes>,
        vault: Arc<Mutex<dyn ProfileVault>>,
    ) -> OckamResult<Profile> {
        let attributes = attributes.unwrap_or(ProfileEventAttributes::new());
        let event = ProfileEvent::new(false, attributes, None, vault.clone())?;

        let identifier: String;
        if let Some(public_key) = event.public_key() {
            let vault = vault.lock().unwrap();
            let hash = vault.sha256(&public_key)?;
            identifier = format!("P_ID.{}", hex::encode(&hash));
        } else {
            return Err(Error::InvalidInternalState.into());
        }
        let mut events: Vec<ProfileEvent> = Vec::with_capacity(1);
        events.push(event);
        let profile = Profile::new(identifier, events, vault);

        Ok(profile)
    }

    pub fn get_profile_public_key(&self, profile: &Profile) -> OckamResult<Option<Vec<u8>>> {
        profile.public_key()
    }

    pub fn rotate_profile(
        &self,
        profile: &mut Profile,
        attributes: Option<ProfileEventAttributes>,
    ) -> OckamResult<()> {
        let attributes = attributes.unwrap_or(ProfileEventAttributes::new());
        profile.rotate(attributes)
    }

    pub fn revoke_profile(
        &self,
        mut profile: Profile,
        attributes: Option<ProfileEventAttributes>,
    ) -> OckamResult<()> {
        let attributes = attributes.unwrap_or(ProfileEventAttributes::new());
        profile.revoke(attributes)?;
        self.delete_profile(profile)
    }

    pub fn attest_profile(&self, profile: &Profile, nonce: &[u8]) -> OckamResult<[u8; 64]> {
        profile.attest(nonce)
    }

    pub fn delete_profile(&self, mut profile: Profile) -> OckamResult<()> {
        profile.delete()
    }
}
