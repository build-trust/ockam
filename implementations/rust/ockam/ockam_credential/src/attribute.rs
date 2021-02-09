use super::structs::*;
use crate::{attribute_type::AttributeType, serde::*};
use serde::{Deserialize, Serialize};

/// An attribute describes a statement that the issuer of a credential is
/// signing about the subject of the credential.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Attribute {
    /// A label for the attribute.
    #[serde(
        serialize_with = "write_byte_string",
        deserialize_with = "read_byte_string"
    )]
    pub label: ByteString,

    /// A longer description of the meaning of the attribute.
    #[serde(
        serialize_with = "write_byte_string",
        deserialize_with = "read_byte_string"
    )]
    pub description: ByteString,

    /// The data type of the attribute value.
    pub attribute_type: AttributeType,
}
