use ockam_vault::KeyId;

use crate::TimestampInSeconds;

/// Options to create an Identity key
pub struct IdentityOptions {
    pub(super) key: KeyId,
    pub(super) revoke_all_purpose_keys: bool,
    pub(super) created_at: TimestampInSeconds,
    pub(super) expires_at: TimestampInSeconds,
}

impl IdentityOptions {
    /// Constructor
    pub fn new(
        key: KeyId,
        revoke_all_purpose_keys: bool,
        created_at: TimestampInSeconds,
        expires_at: TimestampInSeconds,
    ) -> Self {
        Self {
            key,
            revoke_all_purpose_keys,
            created_at,
            expires_at,
        }
    }

    /// New key
    pub fn key(&self) -> &KeyId {
        &self.key
    }

    /// Revoke all PurposeKeys issued by previous Identity keys
    pub fn revoke_all_purpose_keys(&self) -> bool {
        self.revoke_all_purpose_keys
    }

    /// Creation timestamp
    pub fn created_at(&self) -> TimestampInSeconds {
        self.created_at
    }

    /// Expiration timestamp
    pub fn expires_at(&self) -> TimestampInSeconds {
        self.expires_at
    }
}
