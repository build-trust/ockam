use ockam_core::compat::sync::Arc;
use ockam_core::Result;
use ockam_vault::{SigningKeyType, SigningSecretKeyHandle};

use crate::models::{PurposePublicKey, TimestampInSeconds};
use crate::purpose_keys::Ttl;
use crate::{CredentialPurposeKey, Identifier, Purpose, PurposeKeyCreation};

/// Default TTL for an Identity key
pub const DEFAULT_CREDENTIAL_PURPOSE_KEY_TTL: TimestampInSeconds =
    TimestampInSeconds(5 * 365 * 24 * 60 * 60); // Five years

enum Key {
    Generate(SigningKeyType),
    Existing(SigningSecretKeyHandle),
}

/// Builder for [`CredentialPurposeKey`]
pub struct CredentialPurposeKeyBuilder {
    purpose_keys_creation: Arc<PurposeKeyCreation>,

    identifier: Identifier,
    key: Key,
    ttl: Ttl,
}

impl CredentialPurposeKeyBuilder {
    /// Constructor
    pub fn new(purpose_keys_creation: Arc<PurposeKeyCreation>, identifier: Identifier) -> Self {
        let key = Key::Generate(SigningKeyType::EdDSACurve25519);

        Self {
            purpose_keys_creation,
            identifier,
            key,
            ttl: Ttl::CreatedNowWithTtl(DEFAULT_CREDENTIAL_PURPOSE_KEY_TTL),
        }
    }

    /// Use an existing key for the Identity (should be present in the corresponding Vault)
    pub fn with_existing_key(mut self, secret_key_handle: SigningSecretKeyHandle) -> Self {
        self.key = Key::Existing(secret_key_handle);

        self
    }

    /// Will generate a fresh key with the given type
    pub fn with_random_key(mut self, key_type: SigningKeyType) -> Self {
        self.key = Key::Generate(key_type);
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
    pub async fn build(self) -> Result<CredentialPurposeKey> {
        // TODO: Check if such key already exists and rewrite it correctly (also delete from the Vault)

        let purpose_keys_creation = self.purpose_keys_creation.clone();

        let secret_key = match self.key {
            Key::Generate(stype) => {
                purpose_keys_creation
                    .vault()
                    .credential_vault
                    .generate_signing_secret_key(stype)
                    .await?
            }
            Key::Existing(key) => key,
        };

        let (created_at, expires_at) = self.ttl.build()?;

        let public_key = purpose_keys_creation
            .vault()
            .credential_vault
            .get_verifying_public_key(&secret_key)
            .await?;

        let (attestation, attestation_data) = purpose_keys_creation
            .attest_purpose_key(
                self.identifier.clone(),
                PurposePublicKey::CredentialSigning(public_key.clone().into()),
                created_at,
                expires_at,
            )
            .await?;

        purpose_keys_creation
            .repository()
            .set_purpose_key(&self.identifier, Purpose::Credentials, &attestation)
            .await?;

        let purpose_key = CredentialPurposeKey::new(
            self.identifier,
            secret_key,
            public_key,
            attestation_data,
            attestation,
        );

        Ok(purpose_key)
    }
}
