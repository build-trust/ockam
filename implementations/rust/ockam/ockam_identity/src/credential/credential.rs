use crate::CredentialData;
use core::fmt;
use minicbor::{Decode, Encode};
use ockam_core::compat::{string::String, vec::Vec};
use ockam_core::Result;
use serde::de::Error;
use serde::{Deserialize, Deserializer};
use serde::{Serialize, Serializer};
#[cfg(feature = "std")]
use std::ops::Deref;

#[cfg(feature = "tag")]
use crate::TypeTag;
use crate::Unverified;

/// Credential data + signature for that data
#[derive(Clone, Debug, Decode, Encode, PartialEq, Eq)]
#[rustfmt::skip]
#[cbor(map)]
pub struct Credential {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<3796735>,
    /// CBOR-encoded [`CredentialData`].
    #[cbor(with = "minicbor::bytes")]
    #[b(1)] pub data: Vec<u8>,
    /// Cryptographic signature of attributes data.
    #[cbor(with = "minicbor::bytes")]
    #[b(2)] pub signature: Vec<u8>,
}

impl Credential {
    /// Return the signature of a credential
    pub fn signature(&self) -> &[u8] {
        &self.signature
    }

    /// Return the serialized data of a credential
    pub fn unverified_data(&self) -> &[u8] {
        &self.data
    }

    pub(crate) fn new(data: Vec<u8>, signature: Vec<u8>) -> Self {
        Credential {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            data,
            signature,
        }
    }
}

impl fmt::Display for Credential {
    #[cfg(feature = "std")]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let data = CredentialData::<Unverified>::try_from(self)
            .map_err(|_| fmt::Error)?
            .into_verified();
        write!(f, "{}", data)?;
        writeln!(f, "Signature:  {}", hex::encode(self.signature.deref()))
    }

    #[cfg(not(feature = "std"))]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Credential {{ ... }}")
    }
}

impl Serialize for Credential {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        let bytes = minicbor::to_vec(self).expect("encoding credential to vec never errors");
        if ser.is_human_readable() {
            ser.serialize_str(&hex::encode(&bytes))
        } else {
            ser.serialize_bytes(&bytes)
        }
    }
}

impl<'a> Deserialize<'a> for Credential {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'a>,
    {
        let bytes: Vec<u8> = if deserializer.is_human_readable() {
            let s = String::deserialize(deserializer)?;
            hex::decode(s).map_err(D::Error::custom)?
        } else {
            <Vec<u8>>::deserialize(deserializer)?
        };
        minicbor::decode(&bytes).map_err(D::Error::custom)
    }
}

impl TryFrom<&Credential> for CredentialData<Unverified> {
    type Error = minicbor::decode::Error;

    fn try_from(value: &Credential) -> Result<Self, Self::Error> {
        minicbor::decode(value.clone().data.as_slice())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen};
    use quickcheck_macros::quickcheck;
    use serde_json;

    #[quickcheck]
    fn test_serialization_roundtrip(credential: Credential) -> bool {
        let serialized = serde_bare::to_vec(&credential).unwrap();
        let actual: Credential = serde_bare::from_slice(serialized.as_slice()).unwrap();
        actual == credential
    }

    #[test]
    fn test_serialization() {
        // this test makes sure that we are using the minicbor Bytes encoder
        // for the Credential fields
        let credential = Credential::new(vec![1, 2, 3], vec![5, 6, 7]);
        let serialized = serde_bare::to_vec(&credential).unwrap();
        let expected: Vec<u8> = vec![11, 162, 1, 67, 1, 2, 3, 2, 67, 5, 6, 7];
        assert_eq!(serialized, expected)
    }

    #[quickcheck]
    fn test_serialization_roundtrip_human_readable(credential: Credential) -> bool {
        let serialized = serde_json::to_string(&credential).unwrap();
        let actual: Credential = serde_json::from_str(serialized.as_str()).unwrap();
        actual == credential
    }

    #[test]
    fn test_display_credential() {
        let credential_data = super::super::credential_data::test::make_credential_data();
        let data = minicbor::to_vec(credential_data).unwrap();
        let credential = Credential::new(data, vec![1, 2, 3]);

        let actual = format!("{credential}");
        let expected = r#"Schema:     1
Subject:    P6474cfdbf547240b6d716bff89c976810859bc3f47be8ea620df12a392ea6cb7
Issuer:     P0db4fec87ff764485f1311e68d6f474e786f1fdbafcd249a5eb73dd681fd1d5d (OCKAM_RK)
Created:    1970-01-01T00:02:00Z
Expires:    1970-01-01T00:03:20Z
Attributes: {"name": "value"}
Signature:  010203
"#;
        assert_eq!(actual, expected);
    }

    impl Arbitrary for Credential {
        fn arbitrary(g: &mut Gen) -> Self {
            Credential::new(<Vec<u8>>::arbitrary(g), <Vec<u8>>::arbitrary(g))
        }

        /// there is no meaningful shrinking in general for a credential
        fn shrink(&self) -> Box<dyn Iterator<Item = Credential>> {
            Box::new(std::iter::empty())
        }
    }
}
