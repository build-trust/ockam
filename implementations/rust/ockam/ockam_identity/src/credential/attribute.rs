use super::CredentialAttributeType;
use bls12_381_plus::Scalar;
use ockam_core::compat::string::{String, ToString};
use serde::{Deserialize, Serialize};
use signature_core::lib::Message;

/// The attribute data that is signed by
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum CredentialAttribute {
    /// The attribute is allowed to not be specified
    NotSpecified,
    /// The attribute value is specified as empty
    Empty,
    /// The attribute is a UTF-8 String
    String(String),
    /// The attribute is numeric
    Numeric(i64),
    /// The attribute is a sequence of bytes
    Blob([u8; 32]),
}

impl From<i64> for CredentialAttribute {
    fn from(v: i64) -> Self {
        Self::Numeric(v)
    }
}

impl From<&str> for CredentialAttribute {
    fn from(str: &str) -> Self {
        Self::from(str.to_string())
    }
}

impl From<String> for CredentialAttribute {
    fn from(str: String) -> Self {
        Self::String(str)
    }
}

impl From<[u8; 32]> for CredentialAttribute {
    fn from(blob: [u8; 32]) -> Self {
        Self::Blob(blob)
    }
}

impl CredentialAttribute {
    /// Is `self` NotSpecified or Empty
    pub fn can_be_empty(&self) -> bool {
        matches!(
            *self,
            CredentialAttribute::NotSpecified | CredentialAttribute::Empty
        )
    }

    /// convert the attribute data to a cryptographic value that can be signed
    pub fn to_signature_message(&self) -> Message {
        match self {
            CredentialAttribute::NotSpecified => Message(Scalar::one()),
            CredentialAttribute::Empty => Message(Scalar::from_raw([2, 0, 0, 0])),
            CredentialAttribute::Blob(v) => Message::from_bytes(v).unwrap(),
            CredentialAttribute::String(s) => Message::hash(s.as_bytes()),
            CredentialAttribute::Numeric(n) => {
                let f_2_254: Scalar = Scalar::from_raw([
                    0x0000_0000_0000_0000u64,
                    0x0000_0000_0000_0000u64,
                    0x0000_0000_0000_0000u64,
                    0x0200_0000_0000_0000u64,
                ]);
                let d = Scalar::from_raw([*n as u64, 0, 0, 0]);
                if *n < 0 {
                    Message(f_2_254 - d)
                } else {
                    Message(f_2_254 + d)
                }
            }
        }
    }
}

impl PartialEq<CredentialAttributeType> for CredentialAttribute {
    fn eq(&self, other: &CredentialAttributeType) -> bool {
        match (other, self) {
            (&CredentialAttributeType::Blob, &CredentialAttribute::Blob(_)) => true,
            (&CredentialAttributeType::Number, &CredentialAttribute::Numeric(_)) => true,
            (&CredentialAttributeType::Utf8String, &CredentialAttribute::String(_)) => true,
            (_, _) => false,
        }
    }
}
