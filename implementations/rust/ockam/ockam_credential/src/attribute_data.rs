use crate::structs::*;
use crate::AttributeType;
use serde::{Deserialize, Serialize};

#[cfg(feature = "std")]
mod fields {
    pub use bbs::prelude::*;
    pub use ff::{Field, PrimeField};
    pub use pairing_plus::bls12_381::{Fr, FrRepr};
}

/// The attribute data that is signed by
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum AttributeData {
    /// The attribute is allowed to not be specified
    NotSpecified,
    /// The attribute value is specified as empty
    Empty,
    /// The attribute is a UTF-8 String
    String(ByteString),
    /// The attribute is numeric
    Numeric(i64),
    /// The attribute is a sequence of bytes
    Blob([u8; 32]),
}

impl AttributeData {
    /// Is `self` NotSpecified or Empty
    pub fn can_be_empty(&self) -> bool {
        match *self {
            AttributeData::NotSpecified | AttributeData::Empty => true,
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
            AttributeData::NotSpecified => {
                SignatureMessage::from(Fr::from_repr(FrRepr::from(1u64)).unwrap())
            }
            AttributeData::Empty => {
                SignatureMessage::from(Fr::from_repr(FrRepr::from(2u64)).unwrap())
            }
            AttributeData::Blob(v) => SignatureMessage::from(v),
            AttributeData::String(s) => SignatureMessage::hash(s),
            AttributeData::Numeric(n) => {
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

impl PartialEq<AttributeType> for AttributeData {
    fn eq(&self, other: &AttributeType) -> bool {
        match (other, self) {
            (&AttributeType::Blob, &AttributeData::Blob(_)) => true,
            (&AttributeType::Number, &AttributeData::Numeric(_)) => true,
            (&AttributeType::Utf8String, &AttributeData::String(_)) => true,
            (_, _) => false,
        }
    }
}
