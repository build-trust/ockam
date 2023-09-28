use ockam_vault::X25519SecretKeyHandle;
use crate::purpose_keys::options::Ttl;
use crate::TimestampInSeconds;

/// Default TTL for an Identity key
pub const DEFAULT_SECURE_CHANNEL_PURPOSE_KEY_TTL: TimestampInSeconds =
    TimestampInSeconds(5 * 365 * 24 * 60 * 60); // Five years


pub struct SecureChannelPurposeKeyOptions {
    pub(crate) key: SecureChannelPurposeKeyOptionsKey,
    pub(crate) ttl: Ttl,
}

impl SecureChannelPurposeKeyOptions {
    pub fn new() -> Self {
        Self {
            key: SecureChannelPurposeKeyOptionsKey::Generate,
            ttl: Ttl::CreatedNowWithTtl(DEFAULT_SECURE_CHANNEL_PURPOSE_KEY_TTL),
        }
    }

    /// Use an existing key for the Identity (should be present in the corresponding Vault)
    pub fn with_existing_key(mut self, secret_key_handle: X25519SecretKeyHandle) -> Self {
        self.key = SecureChannelPurposeKeyOptionsKey::Existing(secret_key_handle);

        self
    }

    /// Will generate a fresh key with the given type
    pub fn with_random_key(mut self) -> Self {
        self.key = SecureChannelPurposeKeyOptionsKey::Generate;
        self
    }

    /// Set created_at and expires_at timestamps
    pub fn with_timestamps(
        mut self,
        created_at: TimestampInSeconds,
        expires_at: TimestampInSeconds,
    ) -> Self {
        self.ttl = Ttl::FullTimestamps {
            created_at,
            expires_at,
        };
        self
    }

    /// Will set created_at to now and compute expires_at given the TTL
    pub fn with_ttl(mut self, ttl_seconds: impl Into<TimestampInSeconds>) -> Self {
        self.ttl = Ttl::CreatedNowWithTtl(ttl_seconds.into());
        self
    }
}

pub enum SecureChannelPurposeKeyOptionsKey {
    Generate,
    Existing(X25519SecretKeyHandle),
}
