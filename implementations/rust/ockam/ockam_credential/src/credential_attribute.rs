use crate::CredentialAttributeType;
use ockam_core::lib::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "std")]
mod fields {
    pub use bbs::prelude::*;
    pub use ff::{Field, PrimeField};
    pub use pairing_plus::bls12_381::{Fr, FrRepr};
}

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

impl CredentialAttribute {
    /// Is `self` NotSpecified or Empty
    pub fn can_be_empty(&self) -> bool {
        match *self {
            CredentialAttribute::NotSpecified | CredentialAttribute::Empty => true,
            _ => false,
        }
    }

    #[cfg(feature = "std")]
    /// convert the attribute data to a cryptographic value that can be signed
    pub fn to_signature_message(&self) -> fields::SignatureMessage {
        use fields::*;
        // unwraps are okay here since the values are in the finite field
        let f_2_254: Fr = Fr::from_repr(FrRepr([
            0x0000_0000_0000_0000u64,
            0x0000_0000_0000_0000u64,
            0x0000_0000_0000_0000u64,
            0x0200_0000_0000_0000u64,
        ]))
        .unwrap();

        match self {
            CredentialAttribute::NotSpecified => {
                SignatureMessage::from(Fr::from_repr(FrRepr::from(1u64)).unwrap())
            }
            CredentialAttribute::Empty => {
                SignatureMessage::from(Fr::from_repr(FrRepr::from(2u64)).unwrap())
            }
            CredentialAttribute::Blob(v) => SignatureMessage::from(v),
            CredentialAttribute::String(s) => SignatureMessage::hash(s),
            CredentialAttribute::Numeric(n) => {
                let d = Fr::from_repr(FrRepr::from(*n as u64)).unwrap();
                let mut m = f_2_254;
                if *n < 0 {
                    m.sub_assign(&d);
                } else {
                    m.add_assign(&d);
                }
                SignatureMessage::from(m)
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
