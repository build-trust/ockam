use ockam_core::compat::string::{String, ToString};
use ockam_vault::SecretAttributes;
use ockam_vault_core::{SecretPersistence, SecretType, CURVE25519_SECRET_LENGTH};
use serde::{Deserialize, Serialize};

/// Meta-Attributes about a key
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub enum MetaKeyAttributes {
    None,
    SecretAttributes(SecretAttributes),
}

/// Attributes that are used to identify key
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct KeyAttributes {
    label: String,
    meta: MetaKeyAttributes,
}

impl From<&str> for KeyAttributes {
    fn from(str: &str) -> Self {
        Self::new(str.to_string())
    }
}

impl KeyAttributes {
    /// Human-readable key name
    pub fn label(&self) -> &str {
        &self.label
    }
    pub fn meta(&self) -> &MetaKeyAttributes {
        &self.meta
    }
}

impl KeyAttributes {
    /// Create new key attributes
    pub fn new<S: Into<String>>(label: S) -> Self {
        Self {
            label: label.into(),
            meta: MetaKeyAttributes::SecretAttributes(SecretAttributes::new(
                SecretType::Curve25519,
                SecretPersistence::Persistent,
                CURVE25519_SECRET_LENGTH,
            )),
        }
    }

    pub fn with_attributes(label: String, meta: MetaKeyAttributes) -> Self {
        Self { label, meta }
    }
}
