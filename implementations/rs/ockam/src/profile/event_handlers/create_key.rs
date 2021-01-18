use crate::profile::change_event::{
    ChangeEvent, ChangeEventType, CreateKeyEvent, ProfileKeyPurpose, ProfileKeyType,
};
use crate::profile::error::Error;
use crate::profile::profile::Profile;
use crate::profile::profile_manager::ProfileManager;
use crate::profile::signed_change_event::{Signature, SignatureType, SignedChangeEvent};
use crate::profile::{EventId, ProfileEventAttributes, ProfileVault};
use ockam_common::error::OckamResult;
use ockam_vault::types::{SecretAttributes, SecretPersistence, SecretType};
use ockam_vault::Secret;
use std::sync::{Arc, Mutex};

impl ProfileManager {
    pub(crate) fn create_profile_key_event(
        &self,
        profile: &mut Profile,
        key_type: ProfileKeyType,
        key_purpose: ProfileKeyPurpose,
        attributes: Option<ProfileEventAttributes>,
        vault: Arc<Mutex<dyn ProfileVault>>,
    ) -> OckamResult<(SignedChangeEvent, Option<Arc<Mutex<Box<dyn Secret>>>>)> {
        let attributes = attributes.unwrap_or(ProfileEventAttributes::new());

        // Creating key after it was revoked is forbidden
        if profile.find_last_key_event(key_type, key_purpose).is_ok() {
            return Err(Error::InvalidInternalState.into());
        }

        let prev_id = profile.get_last_event_id()?;

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
        let change_event =
            ChangeEvent::new(1, prev_id, attributes, ChangeEventType::CreateKey(event));
        let change_event_binary =
            serde_bare::to_vec(&change_event).map_err(|_| Error::BareError.into())?;

        let event_id = v.sha256(&change_event_binary)?;

        let self_signature = v.sign(&private_key, &event_id)?;
        let self_signature = Signature::new(SignatureType::SelfSign, self_signature);

        let event_id = EventId::from_hash(&event_id);

        let signed_change_event = SignedChangeEvent::new(
            1,
            event_id,
            change_event_binary,
            change_event,
            vec![self_signature],
        );

        Ok((signed_change_event, Some(Arc::new(Mutex::new(private_key)))))
    }
}
