use ockam_core::compat::sync::Arc;
use ockam_core::Result;
use ockam_vault::{SigningKeyType, SigningSecretKeyHandle};

use crate::models::TimestampInSeconds;
use crate::utils::now;
use crate::IdentityOptions;
use crate::{Identifier, IdentitiesCreation};

/// Default TTL for an Identity key
pub const DEFAULT_IDENTITY_TTL: TimestampInSeconds = TimestampInSeconds(10 * 365 * 24 * 60 * 60); // Ten years

enum Key {
    Generate(SigningKeyType),
    Existing(SigningSecretKeyHandle),
}

enum Ttl {
    CreatedNowWithTtl(TimestampInSeconds),
    FullTimestamps {
        attestations_valid_from: TimestampInSeconds,
        attestations_valid_until: TimestampInSeconds,
    },
}

/// Builder for [`Identity`](crate::Identity)
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

    /// Use an existing key for the Identity (should be present in the corresponding [`SigningVault`](ockam_vault::VaultForSigning))
    pub fn with_existing_key(mut self, signing_secret_key_handle: SigningSecretKeyHandle) -> Self {
        self.key = Key::Existing(signing_secret_key_handle);
        self
    }

    /// Will generate a fresh key with the given type
    pub fn with_random_key(mut self, key_type: SigningKeyType) -> Self {
        self.key = Key::Generate(key_type);
        self
    }

    /// Set attestations_valid_from and attestations_valid_until timestamps
    pub fn with_timestamps(
        mut self,
        attestations_valid_from: TimestampInSeconds,
        attestations_valid_until: TimestampInSeconds,
    ) -> Self {
        self.ttl = Ttl::FullTimestamps {
            attestations_valid_from,
            attestations_valid_until,
        };
        self
    }

    /// Will set attestations_valid_from to now and compute attestations_valid_until given the TTL
    pub fn with_ttl(mut self, ttl_seconds: impl Into<TimestampInSeconds>) -> Self {
        self.ttl = Ttl::CreatedNowWithTtl(ttl_seconds.into());
        self
    }

    /// Revoke all previously issued [`PurposeKeys`](crate::PurposeKeys)
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

        let (attestations_valid_from, attestations_valid_until) = match self.ttl {
            Ttl::CreatedNowWithTtl(ttl) => {
                let attestations_valid_from = now()?;
                let attestations_valid_until = attestations_valid_from + ttl;

                (attestations_valid_from, attestations_valid_until)
            }
            Ttl::FullTimestamps {
                attestations_valid_from,
                attestations_valid_until,
            } => (attestations_valid_from, attestations_valid_until),
        };

        let options = IdentityOptions::new(
            key,
            self.revoke_all_purpose_keys,
            attestations_valid_from,
            attestations_valid_until,
        );

        Ok(options)
    }

    /// Create the corresponding [`Identity`](crate::Identity)
    pub async fn build(self) -> Result<Identifier> {
        let identities_creation = self.identities_creation.clone();

        let options = self.build_options().await?;

        identities_creation
            .create_identity_with_options(options)
            .await
    }
}
