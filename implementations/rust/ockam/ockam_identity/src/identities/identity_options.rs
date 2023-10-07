use crate::TimestampInSeconds;
use ockam_vault::SigningSecretKeyHandle;

/// Options to create an Identity key
pub struct IdentityOptions {
    pub(super) signing_secret_key_handle: SigningSecretKeyHandle,
    pub(super) revoke_all_purpose_keys: bool,
    pub(super) created_at: TimestampInSeconds,
    pub(super) expires_at: TimestampInSeconds,
}

impl IdentityOptions {
    /// Constructor
    pub fn new(
        signing_secret_key_handle: SigningSecretKeyHandle,
        revoke_all_purpose_keys: bool,
        created_at: TimestampInSeconds,
        expires_at: TimestampInSeconds,
    ) -> Self {
        Self {
            signing_secret_key_handle,
            revoke_all_purpose_keys,
            created_at,
            expires_at,
        }
    }

    /// New key
    pub fn signing_secret_key_handle(&self) -> &SigningSecretKeyHandle {
        &self.signing_secret_key_handle
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
