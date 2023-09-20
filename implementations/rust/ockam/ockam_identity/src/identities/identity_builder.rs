use ockam_core::compat::sync::Arc;
use ockam_core::Result;
use ockam_vault::{SigningKeyType, SigningSecretKeyHandle};

use crate::models::TimestampInSeconds;
use crate::utils::now;
use crate::IdentitiesCreation;
use crate::{Identity, IdentityOptions};

/// Default TTL for an Identity key
pub const DEFAULT_IDENTITY_TTL: TimestampInSeconds = TimestampInSeconds(10 * 365 * 24 * 60 * 60); // Ten years

enum Key {
    Generate(SigningKeyType),
    Existing(SigningSecretKeyHandle),
}

enum Ttl {
    CreatedNowWithTtl(TimestampInSeconds),
    FullTimestamps {
        created_at: TimestampInSeconds,
        expires_at: TimestampInSeconds,
    },
}

/// Builder for [`Identity`]
pub struct IdentityBuilder {
    identities_creation: Arc<IdentitiesCreation>,

    revoke_all_purpose_keys: bool,
    key: Key,
    ttl: Ttl,
}

impl IdentityBuilder {
    /// Constructor
    pub fn new(identities_creation: Arc<IdentitiesCreation>) -> Self {
        Self {
            identities_creation,
            revoke_all_purpose_keys: false,
            key: Key::Generate(SigningKeyType::EdDSACurve25519),
            ttl: Ttl::CreatedNowWithTtl(DEFAULT_IDENTITY_TTL),
        }
    }

    /// Use an existing key for the Identity (should be present in the corresponding [`SigningVault`])
    pub fn with_existing_key(mut self, signing_secret_key_handle: SigningSecretKeyHandle) -> Self {
        self.key = Key::Existing(signing_secret_key_handle);
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

    /// Revoke all previously issued [`PurposeKey`]s
    pub fn with_purpose_keys_revocation(mut self) -> Self {
        self.revoke_all_purpose_keys = true;
        self
    }

    /// Create the corresponding [`IdentityOptions`] object
    pub async fn build_options(self) -> Result<IdentityOptions> {
        let key = match self.key {
            Key::Generate(stype) => {
                self.identities_creation
                    .identity_vault
                    .generate_signing_secret_key(stype)
                    .await?
            }
            Key::Existing(signing_secret_key_handle) => signing_secret_key_handle,
        };

        let (created_at, expires_at) = match self.ttl {
            Ttl::CreatedNowWithTtl(ttl) => {
                let created_at = now()?;
                let expires_at = created_at + ttl;

                (created_at, expires_at)
            }
            Ttl::FullTimestamps {
                created_at,
                expires_at,
            } => (created_at, expires_at),
        };

        let options =
            IdentityOptions::new(key, self.revoke_all_purpose_keys, created_at, expires_at);

        Ok(options)
    }

    /// Create the corresponding [`Identity`]
    pub async fn build(self) -> Result<Identity> {
        let identities_creation = self.identities_creation.clone();

        let options = self.build_options().await?;

        identities_creation
            .create_identity_with_options(options)
            .await
    }
}
