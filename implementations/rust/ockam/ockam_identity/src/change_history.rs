//! Identity history
use crate::change::IdentityChangeType::{CreateKey, RotateKey};
use crate::change::{IdentityChangeEvent, SignatureType};
use crate::{
    EventIdentifier, IdentityError, IdentityIdentifier, IdentityStateConst, IdentityVault,
};
use minicbor::{Decode, Encode};
use ockam_core::compat::vec::Vec;
use ockam_core::{allow, deny, Encodable, Result};
use ockam_vault::PublicKey;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Debug, Clone, Encode, Decode, PartialEq)]
#[cbor(index_only)]
pub enum IdentityHistoryComparison {
    #[n(1)]
    Equal,
    /// Some changes don't match between current identity and known identity
    #[n(2)]
    Conflict,
    /// Current identity is more recent than known identity
    #[n(3)]
    Newer,
    /// Known identity is more recent
    #[n(4)]
    Older,
}

/// Full history of [`Identity`] changes. History and corresponding secret keys are enough to recreate [`Identity`]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IdentityChangeHistory(Vec<IdentityChangeEvent>);

impl IdentityChangeHistory {
    pub fn export(&self) -> Result<Vec<u8>> {
        serde_bare::to_vec(self).map_err(|_| IdentityError::ConsistencyError.into())
    }

    pub fn import(data: &[u8]) -> Result<Self> {
        let s: Self = serde_bare::from_slice(data).map_err(|_| IdentityError::ConsistencyError)?;

        if !s.check_entire_consistency() {
            return Err(IdentityError::ConsistencyError.into());
        }

        Ok(s)
    }
}

impl IdentityChangeHistory {
    pub(crate) fn new(first_event: IdentityChangeEvent) -> Self {
        Self(vec![first_event])
    }

    pub(crate) fn check_consistency_and_add_event(
        &mut self,
        event: IdentityChangeEvent,
    ) -> Result<()> {
        let slice = core::slice::from_ref(&event);
        if !Self::check_consistency(self.as_ref(), slice) {
            return Err(IdentityError::IdentityVerificationFailed.into());
        }

        self.0.push(event);

        Ok(())
    }
}

impl AsRef<[IdentityChangeEvent]> for IdentityChangeHistory {
    fn as_ref(&self) -> &[IdentityChangeEvent] {
        &self.0
    }
}

// Public API
impl IdentityChangeHistory {
    pub fn compare(&self, known: &Self) -> IdentityHistoryComparison {
        for event_pair in self.0.iter().zip(known.0.iter()) {
            if event_pair.0.identifier() != event_pair.1.identifier() {
                return IdentityHistoryComparison::Conflict;
            }
        }

        match self.0.len().cmp(&known.0.len()) {
            Ordering::Less => IdentityHistoryComparison::Older,
            Ordering::Equal => IdentityHistoryComparison::Equal,
            Ordering::Greater => IdentityHistoryComparison::Newer,
        }
    }

    pub async fn compute_identity_id(
        &self,
        vault: &impl IdentityVault,
    ) -> Result<IdentityIdentifier> {
        let root_public_key = self.get_first_root_public_key()?;

        let key_id = vault
            .compute_key_id_for_public_key(&root_public_key)
            .await?;

        Ok(IdentityIdentifier::from_key_id(key_id))
    }

    pub fn get_public_key(&self, label: &str) -> Result<PublicKey> {
        Self::get_public_key_static(self.as_ref(), label)
    }

    pub fn get_first_root_public_key(&self) -> Result<PublicKey> {
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

    pub fn get_root_public_key(&self) -> Result<PublicKey> {
        self.get_public_key(IdentityStateConst::ROOT_LABEL)
    }

    pub async fn verify_all_existing_events(&self, vault: &impl IdentityVault) -> Result<bool> {
        for i in 0..self.0.len() {
            let existing_events = &self.as_ref()[..i];
            let new_event = &self.as_ref()[i];
            if !Self::verify_event(existing_events, new_event, vault).await? {
                return deny();
            }
        }
        allow()
    }

    /// Check consistency of events that are been added
    pub fn check_entire_consistency(&self) -> bool {
        Self::check_consistency(&[], &self.0)
    }
}

// Pub crate API
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

    pub(crate) fn get_current_root_public_key(
        existing_events: &[IdentityChangeEvent],
    ) -> Result<PublicKey> {
        Self::find_last_key_event_public_key(existing_events, IdentityStateConst::ROOT_LABEL)
    }

    pub(crate) fn get_public_key_static(
        events: &[IdentityChangeEvent],
        label: &str,
    ) -> Result<PublicKey> {
        let event = Self::find_last_key_event(events, label)?;
        event.change_block().change().public_key()
    }

    /// WARNING: This function assumes all existing events in chain are verified.
    /// WARNING: Correctness of events sequence is not verified here.
    pub(crate) async fn verify_event(
        existing_events: &[IdentityChangeEvent],
        new_change_event: &IdentityChangeEvent,
        vault: &impl IdentityVault,
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
