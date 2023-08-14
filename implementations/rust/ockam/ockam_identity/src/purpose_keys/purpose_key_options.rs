use ockam_vault::{KeyId, PublicKey, SecretType};

use crate::{Identifier, Purpose, TimestampInSeconds};

#[derive(Clone)]
/// PurposeKey key.
pub enum PurposeKeyKey {
    /// We have access to the PurposeKey secret to key to then use it
    Secret(KeyId),
    /// Only Public Key accessible, we can still attest such PurposeKey, but won't be able to use it.
    /// The calling side may use corresponding secret key though.
    Public(PublicKey),
}

/// Options to create a Purpose Key
#[derive(Clone)]
pub struct PurposeKeyOptions {
    pub(super) identifier: Identifier,
    pub(super) purpose: Purpose,
    pub(super) key: PurposeKeyKey,
    pub(super) stype: SecretType,
    pub(super) created_at: TimestampInSeconds,
    pub(super) expires_at: TimestampInSeconds,
}

impl PurposeKeyOptions {
    /// Constructor
    pub fn new(
        identifier: Identifier,
        purpose: Purpose,
        key: PurposeKeyKey,
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

    /// Key
    pub fn key(&self) -> &PurposeKeyKey {
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
