use minicbor::{Decode, Encode};
use ockam_core::compat::string::String;
use ockam_core::compat::string::ToString;
use ockam_core::compat::vec::Vec;
use serde::{Deserialize, Serialize};

/// This enum represents the type of values which can be associated to an Identity
/// The `ockam_abac` crate is able to use those attributes in policies to check if an identity
/// is allowed to access a resource
#[derive(Debug, Clone, PartialEq, Encode, Decode, Serialize, Deserialize)]
#[rustfmt::skip]
pub enum AttributeValue {
    /// String value
    #[n(1)] Str   (#[n(0)] String),
    /// Int value
    #[n(2)] Int   (#[n(0)] i64),
    /// Float value
    #[n(3)] Float (#[n(0)] f64),
    /// Bool value
    #[n(4)] Bool  (#[n(0)] bool),
    /// Sequence of other attribute values
    #[n(5)] Seq   (#[n(0)] Vec<AttributeValue>)
}

impl ToString for AttributeValue {
    fn to_string(&self) -> String {
        match self {
            AttributeValue::Str(v) => v.to_string(),
            AttributeValue::Int(v) => v.to_string(),
            AttributeValue::Float(v) => v.to_string(),
            AttributeValue::Bool(v) => v.to_string(),
            AttributeValue::Seq(v) => v
                .iter()
                .map(|a| a.to_string())
                .collect::<Vec<_>>()
                .join(", "),
        }
    }
}

impl From<&str> for AttributeValue {
    fn from(value: &str) -> Self {
        AttributeValue::Str(value.to_string())
    }
}

impl From<String> for AttributeValue {
    fn from(value: String) -> Self {
        AttributeValue::Str(value)
    }
}
