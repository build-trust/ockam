use core::fmt;
use ockam_core::compat::string::String;
use ockam_core::vault::{KeyPersistence, KeyType, CURVE25519_SECRET_LENGTH_U32};
use ockam_vault::KeyAttributes;
use serde::{Deserialize, Serialize};

/// Attributes that are used to identify a key
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct PrivateKeyAttributes {
    label: String,
    key_attributes: KeyAttributes,
}

impl PrivateKeyAttributes {
    /// Human-readable key name
    pub fn label(&self) -> &str {
        &self.label
    }
    /// `KeyAttributes` of the key
    pub fn key_attributes(&self) -> KeyAttributes {
        self.key_attributes
    }
}

impl PrivateKeyAttributes {
    /// Default key with given label. (Ed25519, Persistent)
    pub fn default_with_label(label: impl Into<String>) -> Self {
        Self::new(
            label.into(),
            KeyAttributes::new(
                KeyType::Ed25519,
                KeyPersistence::Persistent,
                CURVE25519_SECRET_LENGTH_U32,
            ),
        )
    }

    /// Constructor
    pub fn new(label: String, key_attributes: KeyAttributes) -> Self {
        Self {
            label,
            key_attributes,
        }
    }
}
impl fmt::Display for PrivateKeyAttributes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            " label:{}, key_attributes:{}",
            self.label(),
            self.key_attributes()
        )
    }
}
