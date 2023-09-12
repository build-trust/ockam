use ockam_core::compat::sync::Arc;
use ockam_core::Result;
use ockam_vault::{KeyId, PublicKey, SecretType};

use crate::models::TimestampInSeconds;
use crate::utils::now;
use crate::{
    Identifier, Purpose, PurposeKey, PurposeKeyKey, PurposeKeyOptions, PurposeKeysCreation,
};

/// Default TTL for an Identity key
pub const DEFAULT_PURPOSE_KEY_TTL: TimestampInSeconds = TimestampInSeconds(5 * 365 * 24 * 60 * 60); // Five years

enum Key {
    Generate(SecretType),
    Existing { key_id: KeyId, stype: SecretType },
    OnlyPublic(PublicKey),
}

enum Ttl {
    CreatedNowWithTtl(TimestampInSeconds),
    FullTimestamps {
        created_at: TimestampInSeconds,
        expires_at: TimestampInSeconds,
    },
}

/// Builder for [`PurposeKey`]
pub struct PurposeKeyBuilder {
    purpose_keys_creation: Arc<PurposeKeysCreation>,

    identifier: Identifier,
    purpose: Purpose,
    key: Key,
    ttl: Ttl,
}

impl PurposeKeyBuilder {
    /// Constructor
    pub fn new(
        purpose_keys_creation: Arc<PurposeKeysCreation>,
        identifier: Identifier,
        purpose: Purpose,
    ) -> Self {
        let key = match purpose {
            Purpose::SecureChannel => Key::Generate(SecretType::X25519),
            Purpose::Credentials => Key::Generate(SecretType::Ed25519),
        };

        Self {
            purpose_keys_creation,
            identifier,
            purpose,
            key,
            ttl: Ttl::CreatedNowWithTtl(DEFAULT_PURPOSE_KEY_TTL),
        }
    }

    /// Use an existing key for the Identity (should be present in the corresponding Vault)
    pub fn with_existing_key(mut self, key_id: KeyId, stype: SecretType) -> Self {
        self.key = Key::Existing { key_id, stype };
        self
    }

    /// Will generate a fresh key with the given type
    pub fn with_random_key(mut self, key_type: SecretType) -> Self {
        self.key = Key::Generate(key_type);
        self
    }

    /// Only public key is available, which is enough to attest it
    /// However, the calling side is then responsible for possession and proper use of the
    /// corresponding secret key
    pub fn with_public_key(mut self, public_key: PublicKey) -> Self {
        self.key = Key::OnlyPublic(public_key);
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

    /// Create the corresponding [`PurposeKeyOptions`] object
    pub async fn build_options(self) -> Result<PurposeKeyOptions> {
        let (key, stype) = match self.key {
            Key::Generate(stype) => {
                let attributes = stype.try_into()?;
                let key = match &self.purpose {
                    Purpose::SecureChannel => {
                        let key_id = self
                            .purpose_keys_creation
                            .vault()
                            .secure_channel_vault
                            .generate_static_secret(attributes)
                            .await?;
                        PurposeKeyKey::Secret(key_id)
                    }
                    Purpose::Credentials => {
                        let key_id = self
                            .purpose_keys_creation
                            .vault()
                            .credential_vault
                            .generate_key(attributes)
                            .await?;
                        PurposeKeyKey::Secret(key_id)
                    }
                };

                (key, stype)
            }
            Key::Existing { key_id, stype } => (PurposeKeyKey::Secret(key_id), stype),
            Key::OnlyPublic(public_key) => {
                let stype = public_key.stype();
                (PurposeKeyKey::Public(public_key), stype)
            }
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

        let options = PurposeKeyOptions::new(
            self.identifier,
            self.purpose,
            key,
            stype,
            created_at,
            expires_at,
        );

        Ok(options)
    }

    /// Create the corresponding [`PurposeKey`]
    pub async fn build(self) -> Result<PurposeKey> {
        let purpose_keys_creation = self.purpose_keys_creation.clone();

        let options = self.build_options().await?;

        purpose_keys_creation
            .create_purpose_key_with_options(options)
            .await
    }
}
