use crate::{credential_attribute_type::CredentialAttributeType, serde::*};
use ockam_core::lib::*;
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
}
