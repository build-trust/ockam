use crate::models::{ChangeData, ChangeHash};
use ockam_vault::PublicKey;

/// Verified Changes of an [`Identity`]
#[derive(Clone, Debug)]
pub struct VerifiedChange {
    data: ChangeData,
    change_hash: ChangeHash,
    primary_public_key: PublicKey,
}

impl VerifiedChange {
    pub(crate) fn new(
        data: ChangeData,
        change_hash: ChangeHash,
        primary_public_key: PublicKey,
    ) -> Self {
        Self {
            data,
            change_hash,
            primary_public_key,
        }
    }

    /// [`ChangeData`]
    pub fn data(&self) -> &ChangeData {
        &self.data
    }

    /// [`ChangeHash`]
    pub fn change_hash(&self) -> &ChangeHash {
        &self.change_hash
    }

    /// [`PrimaryPublicKey`]
    pub fn primary_public_key(&self) -> &PublicKey {
        &self.primary_public_key
    }
}
