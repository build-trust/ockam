use ockam_core::compat::string::String;
use ockam_core::vault::{SecretPersistence, SecretType, CURVE25519_SECRET_LENGTH};
use ockam_vault::SecretAttributes;
use serde::{Deserialize, Serialize};

/// Attributes that are used to identify key
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct KeyAttributes {
    label: String,
    secret_attributes: SecretAttributes,
}

impl KeyAttributes {
    /// Human-readable key name
    pub fn label(&self) -> &str {
        &self.label
    }
    pub fn secret_attributes(&self) -> SecretAttributes {
        self.secret_attributes
    }
}

impl KeyAttributes {
    pub fn default_with_label(label: impl Into<String>) -> Self {
        Self::new(
            label.into(),
            SecretAttributes::new(
                SecretType::Ed25519,
                SecretPersistence::Persistent,
                CURVE25519_SECRET_LENGTH,
            ),
        )
    }

    pub fn new(label: String, secret_attributes: SecretAttributes) -> Self {
        Self {
            label,
            secret_attributes,
        }
    }
}
