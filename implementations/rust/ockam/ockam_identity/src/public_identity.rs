use crate::change_history::{IdentityChangeHistory, IdentityHistoryComparison};
use crate::{IdentityError, IdentityIdentifier, IdentityVault};
use ockam_core::compat::vec::Vec;
use ockam_core::vault::Signature;
use ockam_core::Result;
use ockam_vault::PublicKey;
use serde::{Deserialize, Serialize};

/// Public part of an `Identity`
#[derive(Clone, Serialize, Deserialize)]
pub struct PublicIdentity {
    id: IdentityIdentifier,
    change_history: IdentityChangeHistory,
}

impl PublicIdentity {
    pub(crate) fn new(id: IdentityIdentifier, change_history: IdentityChangeHistory) -> Self {
        Self { id, change_history }
    }

    /// Export to the binary format
    pub fn export(&self) -> Result<Vec<u8>> {
        self.change_history.export()
    }

    /// Import from the binary format
    pub async fn import(data: &[u8], vault: &impl IdentityVault) -> Result<Self> {
        let change_history = IdentityChangeHistory::import(data)?;
        if !change_history.verify_all_existing_changes(vault).await? {
            return Err(IdentityError::IdentityVerificationFailed.into());
        }

        let id = change_history.compute_identity_id(vault).await?;

        let identity = Self::new(id, change_history);

        Ok(identity)
    }

    pub(crate) fn changes(&self) -> &IdentityChangeHistory {
        &self.change_history
    }

    /// Compare to a previously known state of the same `Identity`
    pub fn compare(&self, known: &Self) -> IdentityHistoryComparison {
        self.change_history.compare(&known.change_history)
    }

    /// `IdentityIdentifier`
    pub fn identifier(&self) -> &IdentityIdentifier {
        &self.id
    }

    pub(crate) fn get_root_public_key(&self) -> Result<PublicKey> {
        self.change_history.get_root_public_key()
    }

    pub(crate) fn get_public_key(&self, label: &str) -> Result<PublicKey> {
        self.change_history.get_public_key(label)
    }

    /// Verify signature using key with the given label
    pub async fn verify_signature(
        &self,
        signature: &Signature,
        data: &[u8],
        key_label: Option<&str>,
        vault: &impl IdentityVault,
    ) -> Result<bool> {
        let public_key = match key_label {
            Some(label) => self.get_public_key(label)?,
            None => self.get_root_public_key()?,
        };

        vault.verify(signature, &public_key, data).await
    }
}
