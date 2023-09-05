use ockam_vault::{KeyId, SecretType};

use crate::{Identifier, Purpose, TimestampInSeconds};

/// Options to create an Identity key
pub struct PurposeKeyOptions {
    pub(super) identifier: Identifier,
    pub(super) purpose: Purpose,
    pub(super) key: KeyId,
    pub(super) stype: SecretType,
    pub(super) created_at: TimestampInSeconds,
    pub(super) expires_at: TimestampInSeconds,
}

impl PurposeKeyOptions {
    /// Constructor
    pub fn new(
        identifier: Identifier,
        purpose: Purpose,
        key: KeyId,
        stype: SecretType,
        created_at: TimestampInSeconds,
        expires_at: TimestampInSeconds,
    ) -> Self {
        Self {
            identifier,
            purpose,
            key,
            stype,
            created_at,
            expires_at,
        }
    }

    /// [`Identifier`] of the issuer
    pub fn identifier(&self) -> &Identifier {
        &self.identifier
    }

    /// [`Purpose`]
    pub fn purpose(&self) -> Purpose {
        self.purpose
    }

    /// New key
    pub fn key(&self) -> &KeyId {
        &self.key
    }

    /// Secret key type
    pub fn stype(&self) -> SecretType {
        self.stype
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
