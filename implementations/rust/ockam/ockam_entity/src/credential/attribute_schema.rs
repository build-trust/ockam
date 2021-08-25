use super::*;
use ockam_core::compat::string::{String, ToString};
use serde::{Deserialize, Serialize};

/// An attribute describes a statement that the issuer of a credential is
/// signing about the subject of the credential.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CredentialAttributeSchema {
    /// A label for the attribute.
    #[serde(
        serialize_with = "write_byte_string",
        deserialize_with = "read_byte_string"
    )]
    pub label: String,

    /// A longer description of the meaning of the attribute.
    #[serde(
        serialize_with = "write_byte_string",
        deserialize_with = "read_byte_string"
    )]
    pub description: String,

    /// The data type of the attribute value.
    pub attribute_type: CredentialAttributeType,

    /// If the attribute is allowed to be unknown when signed to the issuer
    pub unknown: bool,
}

impl From<(CredentialAttributeType, &str)> for CredentialAttributeSchema {
    fn from(v: (CredentialAttributeType, &str)) -> Self {
        Self::from((v.0, v.1.to_string()))
    }
}

impl From<(CredentialAttributeType, String)> for CredentialAttributeSchema {
    fn from(v: (CredentialAttributeType, String)) -> Self {
        Self {
            label: v.1,
            description: "".to_string(),
            attribute_type: v.0,
            unknown: false,
        }
    }
}

impl From<&str> for CredentialAttributeSchema {
    fn from(str: &str) -> Self {
        Self::from(str.to_string())
    }
}

impl From<String> for CredentialAttributeSchema {
    fn from(str: String) -> Self {
        Self::from((CredentialAttributeType::Utf8String, str))
    }
}
