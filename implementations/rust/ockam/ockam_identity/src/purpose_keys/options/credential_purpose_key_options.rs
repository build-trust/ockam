use ockam_vault::{SigningKeyType, SigningSecretKeyHandle};
use crate::purpose_keys::options::Ttl;
use crate::TimestampInSeconds;

/// Default TTL for an Identity key
pub const DEFAULT_CREDENTIAL_PURPOSE_KEY_TTL: TimestampInSeconds =
    TimestampInSeconds(5 * 365 * 24 * 60 * 60); // Five years


pub struct CredentialPurposeKeyOptions {
    pub(crate) key: CredentialPurposeKeyOptionsKey,
    pub(crate) ttl: Ttl,
}

impl CredentialPurposeKeyOptions {
    pub fn new() -> Self {
        Self {
            key: CredentialPurposeKeyOptionsKey::Generate(SigningKeyType::EdDSACurve25519),
            ttl: Ttl::CreatedNowWithTtl(DEFAULT_CREDENTIAL_PURPOSE_KEY_TTL),
        }
    }

    /// Use an existing key for the Identity (should be present in the corresponding Vault)
    pub fn with_existing_key(mut self, secret_key_handle: SigningSecretKeyHandle) -> Self {
        self.key = CredentialPurposeKeyOptionsKey::Existing(secret_key_handle);

        self
    }

    /// Will generate a fresh key with the given type
    pub fn with_random_key(mut self, key_type: SigningKeyType) -> Self {
        self.key = CredentialPurposeKeyOptionsKey::Generate(key_type);
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

pub enum CredentialPurposeKeyOptionsKey {
    Generate(SigningKeyType),
    Existing(SigningSecretKeyHandle),
}
