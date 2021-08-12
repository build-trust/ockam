use super::*;
use serde::{Deserialize, Serialize};

/// A schema describes the data format of a credential.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CredentialSchema {
    /// A unique identifier for this schema
    #[serde(
        serialize_with = "write_byte_string",
        deserialize_with = "read_byte_string"
    )]
    pub id: String,

    /// A user friendly name for this schema
    #[serde(
        serialize_with = "write_byte_string",
        deserialize_with = "read_byte_string"
    )]
    pub label: String,

    /// A longer description about this schema
    #[serde(
        serialize_with = "write_byte_string",
        deserialize_with = "read_byte_string"
    )]
    pub description: String,

    /// A list of attributes that are contained in credentials that
    /// have this schema.
    #[serde(
        serialize_with = "write_attributes",
        deserialize_with = "read_attributes"
    )]
    pub attributes: Vec<CredentialAttributeSchema>,
}
