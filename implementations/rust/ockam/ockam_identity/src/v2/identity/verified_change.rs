use super::super::models::ChangeHash;
use ockam_vault::PublicKey;

/// Verified Changes of an [`Identity`]
#[derive(Clone, Debug)]
pub struct VerifiedChange {
    change_hash: ChangeHash,
    primary_public_key: PublicKey,
    revoke_all_purpose_keys: bool,
}

impl VerifiedChange {
    /// [`ChangeHash`]
    pub fn change_hash(&self) -> &ChangeHash {
        &self.change_hash
    }

    /// [`PrimaryPublicKey`]
    pub fn primary_public_key(&self) -> &PublicKey {
        &self.primary_public_key
    }

    /// Are purpose keys revoked with this rotation
    pub fn revoke_all_purpose_keys(&self) -> bool {
        self.revoke_all_purpose_keys
    }

    pub(crate) fn new(
        change_hash: ChangeHash,
        primary_public_key: PublicKey,
        revoke_all_purpose_keys: bool,
    ) -> Self {
        Self {
            change_hash,
            primary_public_key,
            revoke_all_purpose_keys,
        }
    }
}
