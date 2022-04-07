//! Identity history
use crate::IdentityChangeType::{CreateKey, RotateKey};
use crate::{
    EventIdentifier, IdentityChangeEvent, IdentityError, IdentityStateConst, IdentityVault,
    SignatureType,
};
use ockam_core::compat::vec::Vec;
use ockam_core::{allow, deny, Encodable, Result};
use ockam_vault::PublicKey;
use serde::{Deserialize, Serialize};

/// Full history of [`Identity`] changes. History and corresponding secret keys are enough to recreate [`Identity`]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IdentityChangeHistory(Vec<IdentityChangeEvent>);

impl IdentityChangeHistory {
    pub(crate) fn new(change_events: Vec<IdentityChangeEvent>) -> Self {
        Self(change_events)
    }

    pub(crate) fn push_event(&mut self, event: IdentityChangeEvent) {
        self.0.push(event)
    }
}

impl AsRef<[IdentityChangeEvent]> for IdentityChangeHistory {
    fn as_ref(&self) -> &[IdentityChangeEvent] {
        &self.0
    }
}

impl Default for IdentityChangeHistory {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

impl IdentityChangeHistory {
    pub(crate) fn get_last_event_id(&self) -> Result<EventIdentifier> {
        if let Some(e) = self.0.last() {
            Ok(e.identifier().clone())
        } else {
            Err(IdentityError::InvalidInternalState.into())
        }
    }

    pub(crate) fn find_last_key_event<'a>(
        existing_events: &'a [IdentityChangeEvent],
        label: &str,
    ) -> Result<&'a IdentityChangeEvent> {
        existing_events
            .iter()
            .rev()
            .find(|e| e.change_block().change().has_label(label))
            .ok_or_else(|| IdentityError::InvalidInternalState.into())
    }

    pub(crate) fn find_last_key_event_public_key(
        existing_events: &[IdentityChangeEvent],
        label: &str,
    ) -> Result<PublicKey> {
        let last_key_event = Self::find_last_key_event(existing_events, label)?;

        last_key_event.change_block().change().public_key()
    }
}

impl IdentityChangeHistory {
    pub(crate) fn get_current_root_public_key(
        existing_events: &[IdentityChangeEvent],
    ) -> Result<PublicKey> {
        Self::find_last_key_event_public_key(existing_events, IdentityStateConst::ROOT_LABEL)
    }

    pub(crate) fn get_first_root_public_key(&self) -> Result<PublicKey> {
        // TODO: Support root key rotation
        let root_event = match self.as_ref().first() {
            Some(event) => event,
            None => return Err(IdentityError::InvalidInternalState.into()),
        };

        let root_change = root_event.change_block().change();

        let root_create_key_change = match root_change.change_type() {
            CreateKey(c) => c,
            _ => return Err(IdentityError::InvalidInternalState.into()),
        };

        Ok(root_create_key_change.data().public_key().clone())
    }

    pub(crate) fn get_public_key_static(
        events: &[IdentityChangeEvent],
        label: &str,
    ) -> Result<PublicKey> {
        let event = Self::find_last_key_event(events, label)?;
        event.change_block().change().public_key()
    }

    pub(crate) fn get_public_key(&self, label: &str) -> Result<PublicKey> {
        Self::get_public_key_static(self.as_ref(), label)
    }
}

impl IdentityChangeHistory {
    pub(crate) async fn verify_all_existing_events(
        &self,
        vault: &mut impl IdentityVault,
    ) -> Result<bool> {
        for i in 0..self.0.len() {
            let existing_events = &self.as_ref()[..i];
            let new_event = &self.as_ref()[i];
            if !Self::verify_event(existing_events, new_event, vault).await? {
                return deny();
            }
        }
        allow()
    }
    /// WARNING: This function assumes all existing events in chain are verified.
    /// WARNING: Correctness of events sequence is not verified here.
    pub(crate) async fn verify_event(
        existing_events: &[IdentityChangeEvent],
        new_change_event: &IdentityChangeEvent,
        vault: &mut impl IdentityVault,
    ) -> Result<bool> {
        let change_block = new_change_event.change_block();
        let change_block_binary = change_block
            .encode()
            .map_err(|_| IdentityError::BareError)?;

        let event_id = vault.sha256(&change_block_binary).await?;
        let event_id = EventIdentifier::from_hash(event_id);

        if &event_id != new_change_event.identifier() {
            return deny(); // EventIdDoesNotMatch
        }

        struct SignaturesCheck {
            self_sign: u8,
            prev_sign: u8,
            root_sign: u8,
        }

        let mut signatures_check = match new_change_event.change_block().change().change_type() {
            CreateKey(_) => {
                // Should have self signature and root signature
                // There is no Root signature for the very first event
                let root_sign = if existing_events.is_empty() { 0 } else { 1 };

                SignaturesCheck {
                    self_sign: 1,
                    prev_sign: 0,
                    root_sign,
                }
            }
            RotateKey(_) => {
                // Should have self signature, root signature, and previous key signature
                SignaturesCheck {
                    self_sign: 1,
                    prev_sign: 1,
                    root_sign: 1,
                }
            }
        };

        for signature in new_change_event.signatures() {
            let counter;
            let public_key = match signature.stype() {
                SignatureType::RootSign => {
                    if existing_events.is_empty() {
                        return Err(IdentityError::VerifyFailed.into());
                    }

                    counter = &mut signatures_check.root_sign;
                    Self::get_current_root_public_key(existing_events)?
                }
                SignatureType::SelfSign => {
                    counter = &mut signatures_check.self_sign;
                    new_change_event.change_block().change().public_key()?
                }
                SignatureType::PrevSign => {
                    counter = &mut signatures_check.prev_sign;
                    Self::get_public_key_static(
                        existing_events,
                        new_change_event.change_block().change().label(),
                    )?
                }
            };

            if *counter == 0 {
                return Err(IdentityError::VerifyFailed.into());
            }

            if !vault
                .verify(signature.data(), &public_key, event_id.as_ref())
                .await?
            {
                return deny();
            }

            *counter -= 1;
        }

        allow()
    }

    /// Check consistency of events that are been added
    pub(crate) fn check_consistency(
        existing_events: &[IdentityChangeEvent],
        new_events: &[IdentityChangeEvent],
    ) -> bool {
        let mut prev_event = existing_events.last();

        for event in new_events.iter() {
            // Events should go in correct order as stated in previous_event_identifier field
            if let Some(prev) = prev_event {
                if prev.identifier() != event.change_block().previous_event_identifier() {
                    return false; // InvalidChainSequence
                }
            }

            prev_event = Some(event);
        }
        true
    }
}
