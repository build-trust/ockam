use crate::constants::{AES128_SECRET_LENGTH_U32, AES256_SECRET_LENGTH_U32};
use crate::constants::{
    ED25519_SECRET_LENGTH_U32, NIST_P256_SECRET_LENGTH_U32, X25519_SECRET_LENGTH_U32,
};
use core::fmt;
use core::fmt::{Display, Formatter};
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

// TODO: Remove in favor of SecretType
// TODO: Has room for better type-safety
/// Attributes for secrets
///   - a type indicating how the secret is generated: Aes, Ed25519
///   - an expected length corresponding to the type
#[derive(Serialize, Deserialize, Copy, Clone, Debug, Eq, PartialEq)]
#[rustfmt::skip]
pub enum SecretAttributes {
    /// Buffer secret type with user defined length
    Buffer(u32),
    /// Aes secret with length 16
    Aes128,
    /// Aes secret with length 32
    Aes256,
    /// Ed225519 secret with length 32
    Ed25519,
    /// X225519 secret with length 32
    X25519,
    /// NistP256 secret with length 32
    NistP256,
}

impl From<SecretAttributes> for SecretType {
    fn from(value: SecretAttributes) -> Self {
        match value {
            SecretAttributes::Buffer(_) => SecretType::Buffer,
            SecretAttributes::Aes128 => SecretType::Aes,
            SecretAttributes::Aes256 => SecretType::Aes,
            SecretAttributes::Ed25519 => SecretType::Ed25519,
            SecretAttributes::X25519 => SecretType::X25519,
            SecretAttributes::NistP256 => SecretType::NistP256,
        }
    }
}

impl From<SecretType> for SecretAttributes {
    fn from(value: SecretType) -> Self {
        match value {
            SecretType::Buffer => panic!(), // FIXME
            SecretType::Aes => panic!(),    // FIXME
            SecretType::X25519 => SecretAttributes::X25519,
            SecretType::Ed25519 => SecretAttributes::Ed25519,
            SecretType::NistP256 => SecretAttributes::NistP256,
        }
    }
}

impl SecretAttributes {
    /// Return the type of a secret
    pub fn secret_type(&self) -> SecretType {
        (*self).into()
    }

    /// Return the length of a secret
    pub fn length(&self) -> u32 {
        match self {
            SecretAttributes::Buffer(s) => *s,
            SecretAttributes::Aes128 => AES128_SECRET_LENGTH_U32,
            SecretAttributes::Aes256 => AES256_SECRET_LENGTH_U32,
            SecretAttributes::Ed25519 => ED25519_SECRET_LENGTH_U32,
            SecretAttributes::X25519 => X25519_SECRET_LENGTH_U32,
            SecretAttributes::NistP256 => NIST_P256_SECRET_LENGTH_U32,
        }
    }
}

/// All possible [`SecretType`]s
#[derive(Serialize, Deserialize, Copy, Clone, Debug, Encode, Decode, Eq, PartialEq, Zeroize, PartialOrd, Ord)]
#[rustfmt::skip]
#[cbor(index_only)]
pub enum SecretType {
    /// Secret buffer
    #[n(1)] Buffer,
    /// AES key
    #[n(2)] Aes,
    /// Curve 22519 key
    #[n(3)] X25519,
    /// Ed 22519 key
    #[n(4)] Ed25519,
    /// NIST P-256 key
    #[n(5)] NistP256
}

impl Display for SecretType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            SecretType::Buffer => write!(f, "Buffer"),
            SecretType::Aes => write!(f, "Aes"),
            SecretType::X25519 => write!(f, "X25519"),
            SecretType::Ed25519 => write!(f, "Ed25519"),
            SecretType::NistP256 => write!(f, "NistP256"),
        }
    }
}

impl Display for SecretAttributes {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} len:{}", self.secret_type(), self.length())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_deserialize_json() {
        for (attributes, expected_json) in [
            (SecretAttributes::Ed25519, r#""Ed25519""#),
            (SecretAttributes::X25519, r#""X25519""#),
            (SecretAttributes::Buffer(32), r#"{"Buffer":32}"#),
            (SecretAttributes::Aes128, r#""Aes128""#),
            (SecretAttributes::Aes256, r#""Aes256""#),
            (SecretAttributes::NistP256, r#""NistP256""#),
        ] {
            let actual_json = serde_json::to_string(&attributes).unwrap();
            assert_eq!(actual_json, expected_json);

            let actual_attributes: SecretAttributes = serde_json::from_str(expected_json).unwrap();
            assert_eq!(actual_attributes, attributes);
        }
    }

    #[test]
    fn test_serialize_deserialize_bare() {
        for (attributes, expected_bare) in [
            (SecretAttributes::Buffer(32), r#"0020000000"#),
            (SecretAttributes::Aes128, r#"01"#),
            (SecretAttributes::Aes256, r#"02"#),
            (SecretAttributes::Ed25519, r#"03"#),
            (SecretAttributes::X25519, r#"04"#),
            (SecretAttributes::NistP256, r#"05"#),
        ] {
            let actual_bare = hex::encode(serde_bare::to_vec(&attributes).unwrap());
            assert_eq!(actual_bare, expected_bare);

            let actual_attributes: SecretAttributes =
                serde_bare::from_slice(&hex::decode(expected_bare).unwrap()).unwrap();
            assert_eq!(actual_attributes, attributes);
        }
    }
}
