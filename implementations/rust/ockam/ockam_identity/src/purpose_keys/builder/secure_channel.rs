use ockam_core::compat::sync::Arc;
use ockam_core::Result;
use ockam_vault::X25519SecretKeyHandle;

use crate::models::{PurposePublicKey, TimestampInSeconds};
use crate::purpose_keys::Ttl;
use crate::{Identifier, Purpose, PurposeKeyCreation, SecureChannelPurposeKey};

/// Default TTL for an Identity key
pub const DEFAULT_SECURE_CHANNEL_PURPOSE_KEY_TTL: TimestampInSeconds =
    TimestampInSeconds(5 * 365 * 24 * 60 * 60); // Five years

enum Key {
    Generate,
    Existing(X25519SecretKeyHandle),
}

/// Builder for [`SecureChannelPurposeKey`]
pub struct SecureChannelPurposeKeyBuilder {
    purpose_keys_creation: Arc<PurposeKeyCreation>,

    identifier: Identifier,
    key: Key,
    ttl: Ttl,
}

impl SecureChannelPurposeKeyBuilder {
    /// Constructor
    pub fn new(purpose_keys_creation: Arc<PurposeKeyCreation>, identifier: Identifier) -> Self {
        let key = Key::Generate;

        Self {
            purpose_keys_creation,
            identifier,
            key,
            ttl: Ttl::CreatedNowWithTtl(DEFAULT_SECURE_CHANNEL_PURPOSE_KEY_TTL),
        }
    }

    /// Use an existing key for the Identity (should be present in the corresponding Vault)
    pub fn with_existing_key(mut self, secret_key_handle: X25519SecretKeyHandle) -> Self {
        self.key = Key::Existing(secret_key_handle);

        self
    }

    /// Will generate a fresh key with the given type
    pub fn with_random_key(mut self) -> Self {
        self.key = Key::Generate;
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

    /// Create the corresponding [`PurposeKey`]
    pub async fn build(self) -> Result<SecureChannelPurposeKey> {
        // TODO: Check if such key already exists and rewrite it correctly (also delete from the Vault)

        let purpose_keys_creation = self.purpose_keys_creation.clone();

        let secret_key = match self.key {
            Key::Generate => {
                purpose_keys_creation
                    .vault()
                    .secure_channel_vault
                    .generate_static_x25519_secret_key()
                    .await?
            }
            Key::Existing(key) => key,
        };

        let (created_at, expires_at) = self.ttl.build()?;

        let public_key = purpose_keys_creation
            .vault()
            .secure_channel_vault
            .get_x25519_public_key(&secret_key)
            .await?;

        let (attestation, attestation_data) = purpose_keys_creation
            .attest_purpose_key(
                self.identifier.clone(),
                PurposePublicKey::SecureChannelStatic(public_key.clone()),
                created_at,
                expires_at,
            )
            .await?;

        purpose_keys_creation
            .repository()
            .set_purpose_key(&self.identifier, Purpose::SecureChannel, &attestation)
            .await?;

        let purpose_key = SecureChannelPurposeKey::new(
            self.identifier,
            secret_key,
            public_key,
            attestation_data,
            attestation,
        );

        Ok(purpose_key)
    }
}
