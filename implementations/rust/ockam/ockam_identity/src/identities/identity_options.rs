use crate::TimestampInSeconds;
use ockam_vault::SigningSecretKeyHandle;

/// Options to create an Identity key
pub struct IdentityOptions {
    pub(super) signing_secret_key_handle: SigningSecretKeyHandle,
    pub(super) revoke_all_purpose_keys: bool,
    pub(super) attestations_valid_from: TimestampInSeconds,
    pub(super) attestations_valid_until: TimestampInSeconds,
}

impl IdentityOptions {
    /// Constructor
    pub fn new(
        signing_secret_key_handle: SigningSecretKeyHandle,
        revoke_all_purpose_keys: bool,
        attestations_valid_from: TimestampInSeconds,
        attestations_valid_until: TimestampInSeconds,
    ) -> Self {
        Self {
            signing_secret_key_handle,
            revoke_all_purpose_keys,
            attestations_valid_from,
            attestations_valid_until,
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
    pub fn attestations_valid_from(&self) -> TimestampInSeconds {
        self.attestations_valid_from
    }

    /// Expiration timestamp
    pub fn attestations_valid_until(&self) -> TimestampInSeconds {
        self.attestations_valid_until
    }
}
