use minicbor::{Decode, Encode};
use ockam_core::compat::string::String;
use ockam_core::compat::string::ToString;
use serde::{Deserialize, Serialize};

/// This enum represents the type of attributes names which can be associated to an Identity
/// We currently support only String names at the moment but this type opens the possibility
/// of introducing more compact representations of attribute names in the future
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Encode, Decode, Serialize, Deserialize)]
#[rustfmt::skip]
pub enum AttributeName {
    /// String name
    #[n(1)] Str(#[n(0)] String),
}

impl ToString for AttributeName {
    fn to_string(&self) -> String {
        match self {
            AttributeName::Str(v) => v.to_string(),
        }
    }
}

impl From<&str> for AttributeName {
    fn from(value: &str) -> Self {
        AttributeName::Str(value.to_string())
    }
}

impl From<String> for AttributeName {
    fn from(value: String) -> Self {
        AttributeName::Str(value)
    }
}
