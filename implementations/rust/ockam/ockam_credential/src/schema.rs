use super::structs::*;
use crate::{attribute::Attribute, serde::*};
use serde::{Deserialize, Serialize};

/// A schema describes the data format of a credential.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Schema {
    /// A unique identifier for this schema
    #[serde(
        serialize_with = "write_byte_string",
        deserialize_with = "read_byte_string"
    )]
    pub id: ByteString,

    /// A user friendly name for this schema
    #[serde(
        serialize_with = "write_byte_string",
        deserialize_with = "read_byte_string"
    )]
    pub label: ByteString,

    /// A longer description about this schema
    #[serde(
        serialize_with = "write_byte_string",
        deserialize_with = "read_byte_string"
    )]
    pub description: ByteString,

    /// A list of attributes that are contained in credentials that
    /// have this schema.
    #[serde(
        serialize_with = "write_attributes",
        deserialize_with = "read_attributes"
    )]
    pub attributes: Buffer<Attribute>,
}
