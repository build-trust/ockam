use core::fmt;
use minicbor::{Decode, Encode};
use ockam_core::compat::string::String;
use ockam_vault::constants::AES256_SECRET_LENGTH_U32;
use ockam_vault::SecretAttributes;
use ockam_vault::SecretType;
use serde::{Deserialize, Serialize};

/// Attributes that are used to identify a key
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct KeyAttributes {
    label: String,
    secret_attributes: SecretAttributesV1,
}

impl KeyAttributes {
    /// Human-readable key name
    pub fn label(&self) -> &str {
        &self.label
    }
    /// `SecretAttributes` of the key
    pub fn secret_attributes(&self) -> SecretAttributes {
        match self.secret_attributes.stype {
            SecretType::Buffer => SecretAttributes::Buffer(self.secret_attributes.length),
            SecretType::Aes => {
                if self.secret_attributes.length == AES256_SECRET_LENGTH_U32 {
                    SecretAttributes::Aes256
                } else {
                    SecretAttributes::Aes128
                }
            }
            SecretType::X25519 => SecretAttributes::X25519,
            SecretType::Ed25519 => SecretAttributes::Ed25519,
            #[cfg(feature = "rustcrypto")]
            SecretType::NistP256 => SecretAttributes::NistP256,
        }
    }
}

impl KeyAttributes {
    /// Default key with given label (Ed25519)
    pub fn default_with_label(label: impl Into<String>) -> Self {
        Self::new(label.into(), SecretAttributes::Ed25519)
    }

    /// Constructor
    pub fn new(label: String, attributes: SecretAttributes) -> Self {
        Self {
            label,
            secret_attributes: SecretAttributesV1 {
                stype: attributes.secret_type(),
                persistence: SecretPersistence::Persistent,
                length: attributes.length(),
            },
        }
    }
}
impl fmt::Display for KeyAttributes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            " label:{}, secrets:{}",
            self.label(),
            self.secret_attributes
        )
    }
}

/// Attributes for secrets
///   - a type indicating how the secret is generated: Aes, Ed25519
///   - an expected length corresponding to the type
///   - the persistence field is not used anymore but should always be set to Persistent
#[derive(Serialize, Deserialize, Copy, Encode, Decode, Clone, Debug, Eq, PartialEq)]
#[rustfmt::skip]
pub struct SecretAttributesV1 {
    #[n(1)] stype: SecretType,
    #[n(2)] persistence: SecretPersistence,
    #[n(3)] length: u32,
}

impl fmt::Display for SecretAttributesV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?}({:?}) len:{}",
            self.stype, self.persistence, self.length
        )
    }
}

/// This enum is kept for backward compatibility reasons:
///   - it would not possible to remove the persistence field in SecretAttributes because then we can
///     not read secret attributes serialized with serde_bare (since the format is not auto-descriptive)
///   - identities have a signature which is only valid if that field is present in the identity key data
///     if we removed that field, recreating the signature on the change history of an identity would fail
///
#[derive(Serialize, Deserialize, Copy, Clone, Encode, Decode, Debug, Eq, PartialEq)]
#[rustfmt::skip]
#[cbor(index_only)]
pub enum SecretPersistence {
    /// unused
    #[n(1)] Ephemeral,
    /// unused
    #[n(2)] Persistent,
}

impl Default for SecretPersistence {
    fn default() -> Self {
        SecretPersistence::Persistent
    }
}
