//! Identity history
use crate::change::IdentityChange::{CreateKey, RotateKey};
use crate::change::{IdentitySignedChange, SignatureType};
use crate::{
    ChangeIdentifier, IdentityError, IdentityIdentifier, IdentityStateConst, IdentityVault,
};
use core::cmp::Ordering;
use minicbor::{Decode, Encode};
use ockam_core::compat::vec::Vec;
use ockam_core::{allow, deny, Encodable, Result};
use ockam_vault::PublicKey;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq)]
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
pub(crate) struct IdentityChangeHistory(Vec<IdentitySignedChange>);

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
    pub(crate) fn new(first_signed_change: IdentitySignedChange) -> Self {
        Self(vec![first_signed_change])
    }

    pub(crate) fn check_consistency_and_add_change(
        &mut self,
        change: IdentitySignedChange,
    ) -> Result<()> {
        let slice = core::slice::from_ref(&change);
        if !Self::check_consistency(self.as_ref(), slice) {
            return Err(IdentityError::IdentityVerificationFailed.into());
        }

        self.0.push(change);

        Ok(())
    }
}

impl AsRef<[IdentitySignedChange]> for IdentityChangeHistory {
    fn as_ref(&self) -> &[IdentitySignedChange] {
        &self.0
    }
}

impl IdentityChangeHistory {
    pub fn compare(&self, known: &Self) -> IdentityHistoryComparison {
        for change_pair in self.0.iter().zip(known.0.iter()) {
            if change_pair.0.identifier() != change_pair.1.identifier() {
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

        Ok(IdentityIdentifier::from_key_id(&key_id))
    }

    pub fn get_public_key(&self, label: &str) -> Result<PublicKey> {
        Self::get_public_key_static(self.as_ref(), label)
    }

    pub fn get_first_root_public_key(&self) -> Result<PublicKey> {
        // TODO: Support root key rotation
        let root_change = match self.as_ref().first() {
            Some(change) => change,
            None => return Err(IdentityError::InvalidInternalState.into()),
        };

        let root_change = root_change.change();

        let root_create_key_change = match root_change {
            CreateKey(c) => c,
            _ => return Err(IdentityError::InvalidInternalState.into()),
        };

        Ok(root_create_key_change.public_key().clone())
    }

    pub fn get_root_public_key(&self) -> Result<PublicKey> {
        self.get_public_key(IdentityStateConst::ROOT_LABEL)
    }

    pub async fn verify_all_existing_changes(&self, vault: &impl IdentityVault) -> Result<bool> {
        for i in 0..self.0.len() {
            let existing_changes = &self.as_ref()[..i];
            let new_change = &self.as_ref()[i];
            if !Self::verify_change(existing_changes, new_change, vault).await? {
                return deny();
            }
        }
        allow()
    }

    /// Check consistency of changes that are been added
    pub fn check_entire_consistency(&self) -> bool {
        Self::check_consistency(&[], &self.0)
    }
}

// Pub crate API
impl IdentityChangeHistory {
    pub(crate) fn get_last_change_id(&self) -> Result<ChangeIdentifier> {
        if let Some(e) = self.0.last() {
            Ok(e.identifier().clone())
        } else {
            Err(IdentityError::InvalidInternalState.into())
        }
    }

    pub(crate) fn find_last_key_change<'a>(
        existing_changes: &'a [IdentitySignedChange],
        label: &str,
    ) -> Result<&'a IdentitySignedChange> {
        existing_changes
            .iter()
            .rev()
            .find(|&e| e.change().has_label(label))
            .ok_or_else(|| IdentityError::InvalidInternalState.into())
    }

    pub(crate) fn find_last_key_change_public_key(
        existing_changes: &[IdentitySignedChange],
        label: &str,
    ) -> Result<PublicKey> {
        let last_key_change = Self::find_last_key_change(existing_changes, label)?;

        last_key_change.change().public_key()
    }

    pub(crate) fn get_current_root_public_key(
        existing_changes: &[IdentitySignedChange],
    ) -> Result<PublicKey> {
        Self::find_last_key_change_public_key(existing_changes, IdentityStateConst::ROOT_LABEL)
    }

    pub(crate) fn get_public_key_static(
        changes: &[IdentitySignedChange],
        label: &str,
    ) -> Result<PublicKey> {
        let change = Self::find_last_key_change(changes, label)?;
        change.change().public_key()
    }

    /// WARNING: This function assumes all existing changes in chain are verified.
    /// WARNING: Correctness of changes sequence is not verified here.
    pub(crate) async fn verify_change(
        existing_changes: &[IdentitySignedChange],
        new_change: &IdentitySignedChange,
        vault: &impl IdentityVault,
    ) -> Result<bool> {
        let change_binary = new_change
            .change()
            .encode()
            .map_err(|_| IdentityError::BareError)?;

        let change_id = vault.sha256(&change_binary).await?;
        let change_id = ChangeIdentifier::from_hash(change_id);

        if &change_id != new_change.identifier() {
            return deny(); // ChangeIdDoesNotMatch
        }

        struct SignaturesCheck {
            self_sign: u8,
            prev_sign: u8,
            root_sign: u8,
        }

        let mut signatures_check = match new_change.change() {
            CreateKey(_) => {
                // Should have self signature and root signature
                // There is no Root signature for the very first change
                let root_sign = if existing_changes.is_empty() { 0 } else { 1 };

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

        for signature in new_change.signatures() {
            let counter;
            let public_key = match signature.stype() {
                SignatureType::RootSign => {
                    if existing_changes.is_empty() {
                        return Err(IdentityError::VerifyFailed.into());
                    }

                    counter = &mut signatures_check.root_sign;
                    Self::get_current_root_public_key(existing_changes)?
                }
                SignatureType::SelfSign => {
                    counter = &mut signatures_check.self_sign;
                    new_change.change().public_key()?
                }
                SignatureType::PrevSign => {
                    counter = &mut signatures_check.prev_sign;
                    Self::get_public_key_static(existing_changes, new_change.change().label())?
                }
            };

            if *counter == 0 {
                return Err(IdentityError::VerifyFailed.into());
            }

            if !vault
                .verify(signature.data(), &public_key, change_id.as_ref())
                .await?
            {
                return deny();
            }

            *counter -= 1;
        }

        if signatures_check.prev_sign == 0
            && signatures_check.root_sign == 0
            && signatures_check.self_sign == 0
        {
            allow()
        } else {
            deny()
        }
    }

    /// Check consistency of changes that are been added
    pub(crate) fn check_consistency(
        existing_changes: &[IdentitySignedChange],
        new_changes: &[IdentitySignedChange],
    ) -> bool {
        let mut prev_change = existing_changes.last();

        for change in new_changes.iter() {
            // Changes should go in correct order as stated in previous_change_identifier field
            if let Some(prev) = prev_change {
                if prev.identifier() != change.change().previous_change_identifier() {
                    return false; // InvalidChainSequence
                }
            }

            prev_change = Some(change);
        }
        true
    }
}
