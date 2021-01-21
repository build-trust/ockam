use crate::profile::change_event::{
    Change, ChangeEventType, CreateKeyEvent, ProfileKeyPurpose, ProfileKeyType,
};
use crate::profile::error::Error;
use crate::profile::profile::{KeyEntry, Profile};
use crate::profile::signed_change_event::{
    Changes, ProfileChangeEvent, Proof, Signature, SignatureType,
};
use crate::profile::{EventId, ProfileEventAttributes, ProfileId, ProfileVault};
use ockam_common::error::OckamResult;
use ockam_vault::types::{SecretAttributes, SecretPersistence, SecretType};
use std::sync::{Arc, Mutex};

pub struct ProfileManager {}

impl ProfileManager {
    pub fn new() -> Self {
        Self {}
    }

    pub fn create_profile(
        &self,
        key_type: ProfileKeyType,
        key_purpose: ProfileKeyPurpose,
        attributes: Option<ProfileEventAttributes>,
        vault: Arc<Mutex<dyn ProfileVault>>,
    ) -> OckamResult<Profile> {
        let attributes = attributes.unwrap_or(ProfileEventAttributes::new());

        let mut v = vault.lock().unwrap();

        // TODO: Should be customisable
        let secret_attributes = SecretAttributes {
            stype: SecretType::Curve25519,
            persistence: SecretPersistence::Persistent,
            length: 0,
        };

        let private_key = v.secret_generate(secret_attributes)?;
        let public_key = v.secret_public_key_get(&private_key)?;

        let event = CreateKeyEvent::new(key_type, key_purpose, public_key.as_ref().to_vec());
        let prev_id = v.sha256(&[])?;
        let prev_id = EventId::from_hash(prev_id);
        let change = Change::new(1, prev_id, attributes, ChangeEventType::CreateKey(event));
        let changes = Changes::new_single(change);
        let changes_binary = serde_bare::to_vec(&changes).map_err(|_| Error::BareError.into())?;

        let event_id = v.sha256(&changes_binary)?;
        let event_id = EventId::from_hash(event_id);

        let self_signature = v.sign(&private_key, event_id.as_ref())?;
        let self_signature =
            Proof::Signature(Signature::new(SignatureType::SelfSign, self_signature));

        let signed_change_event =
            ProfileChangeEvent::new(1, event_id.clone(), changes, vec![self_signature]);

        let public_kid = v.sha256(public_key.as_ref())?;
        let public_kid = ProfileId::from_hash(public_kid);

        let mut profile = Profile::new(public_kid, Vec::new(), Vec::new(), vault.clone());

        let key_entry = KeyEntry::new(
            event_id,
            key_type,
            key_purpose,
            Arc::new(Mutex::new(private_key)),
        );

        profile.add_event(signed_change_event, vec![key_entry])?;

        Ok(profile)
    }

    pub fn create_profile_key(
        &self,
        profile: &mut Profile,
        key_type: ProfileKeyType,
        key_purpose: ProfileKeyPurpose,
        attributes: Option<ProfileEventAttributes>,
        vault: Arc<Mutex<dyn ProfileVault>>,
    ) -> OckamResult<()> {
        let event =
            self.create_profile_key_event(profile, key_type, key_purpose, attributes, vault)?;
        profile.add_event(event.0, event.1)
    }

    pub fn rotate_profile_key(
        &self,
        profile: &mut Profile,
        key_type: ProfileKeyType,
        key_purpose: ProfileKeyPurpose,
        attributes: Option<ProfileEventAttributes>,
        vault: Arc<Mutex<dyn ProfileVault>>,
    ) -> OckamResult<()> {
        let event =
            self.rotate_profile_key_event(profile, key_type, key_purpose, attributes, vault)?;
        profile.add_event(event.0, event.1)
    }

    pub fn revoke_profile_key(
        &self,
        profile: &mut Profile,
        key_type: ProfileKeyType,
        key_purpose: ProfileKeyPurpose,
        attributes: Option<ProfileEventAttributes>,
        vault: Arc<Mutex<dyn ProfileVault>>,
    ) -> OckamResult<()> {
        let event =
            self.revoke_profile_key_event(profile, key_type, key_purpose, attributes, vault)?;
        profile.add_event(event.0, event.1)
    }

    pub fn get_profile_public_key(
        &self,
        profile: &Profile,
        key_type: ProfileKeyType,
        key_purpose: ProfileKeyPurpose,
    ) -> Option<Vec<u8>> {
        match profile.public_key(key_type, key_purpose) {
            Ok(k) => Some(k.to_vec()),
            Err(_) => None,
        }
    }

    pub fn attest_profile(
        &self,
        profile: &Profile,
        key_type: ProfileKeyType,
        key_purpose: ProfileKeyPurpose,
        nonce: &[u8],
        vault: Arc<Mutex<dyn ProfileVault>>,
    ) -> OckamResult<[u8; 64]> {
        let last_event_id = profile
            .find_last_key_event(key_type, key_purpose)?
            .identifier();
        let private_key = profile.get_private_key(key_type, key_purpose, last_event_id)?;
        let private_key = private_key.lock().unwrap();

        let mut vault = vault.lock().unwrap();

        vault.sign(&private_key, nonce)
    }

    pub fn delete_profile(&self, mut _profile: Profile) -> OckamResult<()> {
        unimplemented!()
    }
}
