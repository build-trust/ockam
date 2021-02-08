use super::structs::*;
use crate::{attribute_type::AttributeType, serdes::*};
use serde::{Deserialize, Serialize};

/// Attributes describe the claims in credentials. The attribute
/// describes the name of the claim,
/// its meaning and how it is cryptographically signed.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Attribute {
    /// The name of the attribute
    #[serde(
        serialize_with = "write_byte_string",
        deserialize_with = "read_byte_string"
    )]
    pub label: ByteString,
    /// A longer description of the meaning of the attribute
    #[serde(
        serialize_with = "write_byte_string",
        deserialize_with = "read_byte_string"
    )]
    pub description: ByteString,
    /// The method that converts the attribute value to a cryptographic field element
    pub attribute_type: AttributeType,
}
