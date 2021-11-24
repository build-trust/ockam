use ockam_core::compat::string::String;
use ockam_vault::SecretAttributes;
use ockam_vault_core::{SecretPersistence, SecretType, CURVE25519_SECRET_LENGTH};
use serde::{Deserialize, Serialize};

/// Meta-Attributes about a key
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub enum MetaKeyAttributes {
    SecretAttributes(SecretAttributes),
}

/// Attributes that are used to identify key
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct KeyAttributes {
    label: String,
    meta: MetaKeyAttributes,
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
    pub fn default_with_label(label: impl Into<String>) -> Self {
        Self::new(
            label.into(),
            MetaKeyAttributes::SecretAttributes(SecretAttributes::new(
                SecretType::Ed25519,
                SecretPersistence::Persistent,
                CURVE25519_SECRET_LENGTH,
            )),
        )
    }

    pub fn new(label: String, meta: MetaKeyAttributes) -> Self {
        Self { label, meta }
    }
}
