use minicbor::{Decode, Encode};
use ockam_core::compat::string::String;
use ockam_core::compat::string::ToString;
use ockam_core::compat::vec::Vec;
#[cfg(feature = "storage")]
use ockam_core::errcode::{Kind, Origin};
#[cfg(feature = "storage")]
use ockam_core::{Error, Result};
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

impl AttributeValue {
    #[cfg(feature = "storage")]
    pub(crate) fn encode_to_string(&self) -> Result<String> {
        serde_json::to_string(self)
            .map_err(|e| Error::new(Origin::Core, Kind::Serialization, e.to_string()))
    }

    #[cfg(feature = "storage")]
    pub(crate) fn decode_from_string(value: &str) -> Result<Self> {
        serde_json::from_str(value)
            .map_err(|e| Error::new(Origin::Core, Kind::Serialization, e.to_string()))
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_string_roundtrip() {
        check_roundtrip(AttributeValue::Str("ockam".into()), "{\"Str\":\"ockam\"}");
        check_roundtrip(AttributeValue::Int(1), "{\"Int\":1}");
        check_roundtrip(AttributeValue::Bool(true), "{\"Bool\":true}");
        check_roundtrip(AttributeValue::Bool(false), "{\"Bool\":false}");
        check_roundtrip(AttributeValue::Float(10.4), "{\"Float\":10.4}");
        check_roundtrip(
            AttributeValue::Seq(vec![
                AttributeValue::Str("ockam".into()),
                AttributeValue::Int(1),
            ]),
            "{\"Seq\":[{\"Str\":\"ockam\"},{\"Int\":1}]}",
        );
    }

    /// HELPERS
    fn check_roundtrip(attribute_value: AttributeValue, encoded: &str) {
        assert_eq!(&attribute_value.encode_to_string().unwrap(), encoded);
        assert_eq!(
            AttributeValue::decode_from_string(encoded).ok(),
            Some(attribute_value)
        );
    }
}
