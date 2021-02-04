use super::structs::*;
use crate::{attribute::Attribute, serdes::*};
use serde::{Deserialize, Serialize};

/// A schema describes the layout of a credential in a similar manner
/// that a schema describes a database table.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Schema {
    /// The schema unique identifier
    #[serde(
        serialize_with = "write_byte_string",
        deserialize_with = "read_byte_string"
    )]
    pub id: ByteString,
    /// The user friendly name of the schema
    #[serde(
        serialize_with = "write_byte_string",
        deserialize_with = "read_byte_string"
    )]
    pub label: ByteString,
    /// A longer description about the schema
    #[serde(
        serialize_with = "write_byte_string",
        deserialize_with = "read_byte_string"
    )]
    pub description: ByteString,
    /// The attributes that are contained in corresponding credentials
    #[serde(
        serialize_with = "write_attributes",
        deserialize_with = "read_attributes"
    )]
    pub attributes: Buffer<Attribute>,
}
