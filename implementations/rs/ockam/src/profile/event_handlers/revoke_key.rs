use crate::profile::change_event::{
    Change, ChangeEventType, ProfileKeyPurpose, ProfileKeyType, RevokeKeyEvent,
};
use crate::profile::error::Error;
use crate::profile::profile::{KeyEntry, Profile};
use crate::profile::profile_manager::ProfileManager;
use crate::profile::signed_change_event::{
    Changes, ProfileChangeEvent, Proof, Signature, SignatureType,
};
use crate::profile::{EventId, ProfileEventAttributes, ProfileVault};
use ockam_common::error::OckamResult;
use std::sync::{Arc, Mutex};

impl ProfileManager {
    pub(crate) fn revoke_profile_key_event(
        &self,
        profile: &mut Profile,
        key_type: ProfileKeyType,
        key_purpose: ProfileKeyPurpose,
        attributes: Option<ProfileEventAttributes>,
        vault: Arc<Mutex<dyn ProfileVault>>,
    ) -> OckamResult<(ProfileChangeEvent, Vec<KeyEntry>)> {
        let attributes = attributes.unwrap_or(ProfileEventAttributes::new());

        let prev_event_id = profile.get_last_event_id()?;
        let last_event_in_chain = profile.find_last_key_event(key_type, key_purpose)?;

        let last_key_in_chain =
            profile.get_private_key(key_type, key_purpose, last_event_in_chain.identifier())?;
        let last_key_in_chain = last_key_in_chain.lock().unwrap();

        let mut v = vault.lock().unwrap();

        let event = RevokeKeyEvent::new(key_type, key_purpose);

        let change = Change::new(
            1,
            prev_event_id,
            attributes.clone(),
            ChangeEventType::RevokeKey(event),
        );
        let changes = Changes::new_single(change);
        let changes_binary = serde_bare::to_vec(&changes).map_err(|_| Error::BareError.into())?;

        let event_id = v.sha256(&changes_binary)?;
        let event_id = EventId::from_hash(event_id);

        let prev_signature = v.sign(&last_key_in_chain, event_id.as_ref())?;
        let prev_signature =
            Proof::Signature(Signature::new(SignatureType::Previous, prev_signature));

        let signed_change_event =
            ProfileChangeEvent::new(1, event_id, changes, vec![prev_signature]);

        Ok((signed_change_event, vec![]))
    }
}
