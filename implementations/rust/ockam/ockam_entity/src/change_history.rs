//! Profile history
use crate::profile::Profile;
use crate::ProfileChangeType::{CreateKey, RotateKey};
use crate::{
    EntityError, EventIdentifier, KeyAttributes, MetaKeyAttributes, ProfileChange,
    ProfileChangeEvent, ProfileChangeProof, ProfileVault, SignatureType,
};
use ockam_core::compat::{string::ToString, vec::Vec};
use ockam_core::{allow, deny};
use ockam_vault::{PublicKey, SecretAttributes};
use ockam_vault_core::{SecretPersistence, SecretType, CURVE25519_SECRET_LENGTH};
use serde::{Deserialize, Serialize};

/// Full history of [`Profile`] changes. History and corresponding secret keys are enough to recreate [`Profile`]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct ProfileChangeHistory(Vec<ProfileChangeEvent>);

impl ProfileChangeHistory {
    pub(crate) fn new(change_events: Vec<ProfileChangeEvent>) -> Self {
        Self(change_events)
    }

    pub(crate) fn push_event(&mut self, event: ProfileChangeEvent) {
        self.0.push(event)
    }
}

impl AsRef<[ProfileChangeEvent]> for ProfileChangeHistory {
    fn as_ref(&self) -> &[ProfileChangeEvent] {
        &self.0
    }
}

impl Default for ProfileChangeHistory {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

impl ProfileChangeHistory {
    pub(crate) fn get_last_event_id(&self) -> ockam_core::Result<EventIdentifier> {
        if let Some(e) = self.0.last() {
            Ok(e.identifier().clone())
        } else {
            Err(EntityError::InvalidInternalState.into())
        }
    }

    pub(crate) fn find_key_change_in_event<'a>(
        event: &'a ProfileChangeEvent,
        key_attributes: &KeyAttributes,
    ) -> Option<&'a ProfileChange> {
        event
            .changes()
            .data()
            .iter()
            .rev()
            .find(|c| match c.change_type() {
                CreateKey(change) => change.data().key_attributes() == key_attributes,
                RotateKey(change) => change.data().key_attributes() == key_attributes, // RevokeKey(event) => {
                                                                                       //     event.key_type() == key_type && event.key_purpose() == key_purpose && event.label() == label
                                                                                       // }
            })
    }

    pub(crate) fn find_last_key_event<'a>(
        existing_events: &'a [ProfileChangeEvent],
        key_attributes: &KeyAttributes,
    ) -> ockam_core::Result<&'a ProfileChangeEvent> {
        existing_events
            .iter()
            .rev()
            .find(|e| Self::find_key_change_in_event(e, key_attributes).is_some())
            .ok_or_else(|| EntityError::InvalidInternalState.into())
    }

    pub(crate) fn find_last_key_event_public_key(
        existing_events: &[ProfileChangeEvent],
        key_attributes: &KeyAttributes,
    ) -> ockam_core::Result<PublicKey> {
        let last_key_event = Self::find_last_key_event(existing_events, key_attributes)?;

        Self::get_public_key_from_event(key_attributes, last_key_event)
    }

    pub(crate) fn get_change_public_key(change: &ProfileChange) -> ockam_core::Result<PublicKey> {
        let data = match change.change_type() {
            CreateKey(change) => change.data().public_key(),
            RotateKey(change) => change.data().public_key(),
        };

        if data.is_empty() {
            Err(EntityError::InvalidInternalState.into())
        } else {
            Ok(PublicKey::new(data.into()))
        }
    }

    pub(crate) fn get_public_key_from_event(
        key_attributes: &KeyAttributes,
        event: &ProfileChangeEvent,
    ) -> ockam_core::Result<PublicKey> {
        let change = Self::find_key_change_in_event(event, key_attributes)
            .ok_or(EntityError::InvalidInternalState)?;

        Self::get_change_public_key(change)
    }
}

impl ProfileChangeHistory {
    pub(crate) fn get_current_profile_update_public_key(
        existing_events: &[ProfileChangeEvent],
    ) -> ockam_core::Result<PublicKey> {
        let key_attributes = KeyAttributes::with_attributes(
            Profile::PROFILE_UPDATE.to_string(),
            MetaKeyAttributes::SecretAttributes(SecretAttributes::new(
                SecretType::Curve25519,
                SecretPersistence::Persistent,
                CURVE25519_SECRET_LENGTH,
            )),
        );
        Self::find_last_key_event_public_key(existing_events, &key_attributes)
    }

    pub(crate) fn get_first_root_public_key(&self) -> ockam_core::Result<PublicKey> {
        // TODO: Support root key rotation
        let root_event;
        if let Some(re) = self.as_ref().first() {
            root_event = re;
        } else {
            return Err(EntityError::InvalidInternalState.into());
        }

        let root_change;
        if let Some(rc) = root_event.changes().data().first() {
            root_change = rc;
        } else {
            return Err(EntityError::InvalidInternalState.into());
        }

        let root_create_key_change;
        if let CreateKey(c) = root_change.change_type() {
            root_create_key_change = c;
        } else {
            return Err(EntityError::InvalidInternalState.into());
        }

        Ok(PublicKey::new(
            root_create_key_change.data().public_key().to_vec(),
        ))
    }

    pub(crate) fn get_public_key(
        &self,
        key_attributes: &KeyAttributes,
    ) -> ockam_core::Result<PublicKey> {
        let event = Self::find_last_key_event(self.as_ref(), key_attributes)?;
        Self::get_public_key_from_event(key_attributes, event)
    }
}

impl ProfileChangeHistory {
    pub(crate) fn verify_all_existing_events(
        &self,
        vault: &mut impl ProfileVault,
    ) -> ockam_core::Result<bool> {
        for i in 0..self.0.len() {
            let existing_events = &self.as_ref()[..i];
            let new_event = &self.as_ref()[i];
            if !Self::verify_event(existing_events, new_event, vault)? {
                return deny();
            }
        }
        allow()
    }
    /// WARNING: This function assumes all existing events in chain are verified.
    /// WARNING: Correctness of events sequence is not verified here.
    pub(crate) fn verify_event(
        existing_events: &[ProfileChangeEvent],
        new_change_event: &ProfileChangeEvent,
        vault: &mut impl ProfileVault,
    ) -> ockam_core::Result<bool> {
        let changes = new_change_event.changes();
        let changes_binary = serde_bare::to_vec(&changes).map_err(|_| EntityError::BareError)?;

        let event_id = vault.sha256(&changes_binary)?;
        let event_id = EventIdentifier::from_hash(event_id);

        if &event_id != new_change_event.identifier() {
            return deny(); // EventIdDoesntMatch
        }

        match new_change_event.proof() {
            ProfileChangeProof::Signature(s) => match s.stype() {
                SignatureType::RootSign => {
                    let events_to_look = if existing_events.is_empty() {
                        core::slice::from_ref(new_change_event)
                    } else {
                        existing_events
                    };
                    let root_public_key =
                        Self::get_current_profile_update_public_key(events_to_look)?;
                    if !vault.verify(s.data(), &root_public_key, event_id.as_ref())? {
                        return deny();
                    }
                }
            },
        }

        for change in new_change_event.changes().data() {
            if !match change.change_type() {
                CreateKey(c) => {
                    // Should have 1 self signature
                    let data_binary =
                        serde_bare::to_vec(c.data()).map_err(|_| EntityError::BareError)?;
                    let data_hash = vault.sha256(data_binary.as_slice())?;

                    // if verification failed, there is no channel back. Return bool msg?
                    vault.verify(
                        c.self_signature(),
                        &PublicKey::new(c.data().public_key().into()),
                        &data_hash,
                    )?
                }
                RotateKey(c) => {
                    // Should have 1 self signature and 1 prev signature
                    let data_binary =
                        serde_bare::to_vec(c.data()).map_err(|_| EntityError::BareError)?;
                    let data_hash = vault.sha256(data_binary.as_slice())?;

                    if !vault.verify(
                        c.self_signature(),
                        &PublicKey::new(c.data().public_key().into()),
                        &data_hash,
                    )? {
                        false
                    } else {
                        let prev_key_event =
                            Self::find_last_key_event(existing_events, c.data().key_attributes())?;
                        let prev_key_change = ProfileChangeHistory::find_key_change_in_event(
                            prev_key_event,
                            c.data().key_attributes(),
                        )
                        .ok_or(EntityError::InvalidInternalState)?;
                        let public_key =
                            ProfileChangeHistory::get_change_public_key(prev_key_change)?;

                        vault.verify(c.prev_signature(), &public_key, &data_hash)?
                    }
                }
            } {
                return Err(EntityError::VerifyFailed.into());
            }
        }

        allow()
    }

    /// Check consistency of events that are been added
    pub(crate) fn check_consistency(
        existing_events: &[ProfileChangeEvent],
        new_events: &[ProfileChangeEvent],
    ) -> bool {
        // TODO: add more checks: e.g. you cannot rotate the same key twice during one event
        let mut prev_event;
        if let Some(e) = existing_events.last() {
            prev_event = Some(e);
        } else {
            prev_event = None;
        }

        for event in new_events.iter() {
            // Events should go in correct order as stated in previous_event_identifier field
            if let Some(prev) = prev_event {
                if prev.identifier() != event.changes().previous_event_identifier() {
                    return false; // InvalidChainSequence
                }
            }

            prev_event = Some(event);

            // For now only allow one change at a time
            if event.changes().data().len() != 1 {
                return false; // InvalidChainSequence
            }
        }
        true
    }
}
