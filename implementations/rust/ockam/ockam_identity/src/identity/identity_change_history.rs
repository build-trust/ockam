//! Identity history
use crate::identity::identity_change::IdentityChange::CreateKey;
use crate::identity::identity_change::{
    ChangeIdentifier, IdentityChangeConstants, IdentitySignedChange,
};
use crate::identity::IdentityError;
use core::cmp::Ordering;
use core::fmt;
use minicbor::{Decode, Encode};
use ockam_core::compat::string::String;
use ockam_core::compat::vec::Vec;
use ockam_core::Result;
use ockam_vault::PublicKey;
use serde::{Deserialize, Serialize};

/// Result of comparison of current `IdentityChangeHistory` to the `IdentityChangeHistory`
/// of the same Identity, that was known to us earlier
#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq)]
#[cbor(index_only)]
pub enum IdentityHistoryComparison {
    /// No difference
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

/// Full history of [`crate::secure_channels::SecureChannels`] changes. History and corresponding secret keys are enough to recreate [`crate::secure_channels::SecureChannels`]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IdentityChangeHistory(Vec<IdentitySignedChange>);

impl fmt::Display for IdentityChangeHistory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Change History:")?;
        for (i_num, ident) in self.0.iter().enumerate() {
            let public_key = ident.change().public_key().unwrap();
            writeln!(f, "  Change[{}]:", i_num)?;
            writeln!(f, "    identifier: {}", ident.identifier())?;
            writeln!(f, "    change:")?;
            writeln!(
                f,
                "      prev_change_identifier: {}",
                ident.change().previous_change_identifier()
            )?;
            writeln!(f, "      label:        {}", ident.change().label())?;
            writeln!(f, "      public_key:   {}", public_key)?;
            writeln!(f, "    signatures:")?;
            for (sig_num, sig) in ident.signatures().iter().enumerate() {
                writeln!(f, "      [{}]: {}", sig_num, sig)?;
            }
        }
        Ok(())
    }
}

impl IdentityChangeHistory {
    /// Export `IdentityChangeHistory` to the binary format
    pub fn export(&self) -> Result<Vec<u8>> {
        serde_bare::to_vec(self).map_err(|_| IdentityError::ConsistencyError.into())
    }

    /// Export `IdentityChangeHistory` to the hex format
    pub fn export_hex(&self) -> Result<String> {
        Ok(hex::encode(self.export()?))
    }

    /// Import `IdentityChangeHistory` from the binary format
    pub fn import(data: &[u8]) -> Result<Self> {
        let s: Self = serde_bare::from_slice(data).map_err(|_| IdentityError::ConsistencyError)?;
        s.check_entire_consistency()?;
        Ok(s)
    }

    /// Import `IdentityChangeHistory` from hex format
    pub fn import_hex(data: &str) -> Result<Self> {
        Self::import(
            hex::decode(data)
                .map_err(|_| IdentityError::ConsistencyError)?
                .as_slice(),
        )
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
    /// Compare current `IdentityChangeHistory` to the `IdentityChangeHistory` of the same Identity,
    /// that was known to us earlier
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

    /// Get public key with the given label (name)
    pub fn get_public_key(&self, label: &str) -> Result<PublicKey> {
        Self::get_public_key_static(self.as_ref(), label)
    }

    /// Get first root public key
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

    /// Get latest root public key
    pub fn get_root_public_key(&self) -> Result<PublicKey> {
        self.get_public_key(IdentityChangeConstants::ROOT_LABEL)
    }

    /// Check consistency of changes that are being added
    pub fn check_entire_consistency(&self) -> Result<()> {
        if !Self::check_consistency(&[], &self.0) {
            return Err(IdentityError::ConsistencyError.into());
        }
        Ok(())
    }
}

// Pub crate API
impl IdentityChangeHistory {
    pub(crate) fn add_change(&mut self, change: IdentitySignedChange) -> Result<()> {
        self.check_consistency_and_add_change(change)
    }

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
        Self::find_last_key_change_public_key(existing_changes, IdentityChangeConstants::ROOT_LABEL)
    }

    pub(crate) fn get_public_key_static(
        changes: &[IdentitySignedChange],
        label: &str,
    ) -> Result<PublicKey> {
        let change = Self::find_last_key_change(changes, label)?;
        change.change().public_key()
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
