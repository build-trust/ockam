use crate::cloud::enroll::auth0::OidcToken;
use core::str::FromStr;
use minicbor::bytes::ByteArray;
use minicbor::{CborLen, Decode, Encode};
use ockam_core::compat::rand;
use ockam_core::compat::rand::RngCore;
use ockam_core::compat::string::{String, ToString};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::Error;
use ockam_core::Result;
use ockam_node::database::{SqlxType, ToSqlxType};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

/// A one-time code can be used to enroll
/// a node with some authenticated attributes
/// It can be retrieve with a command like `ockam project ticket --attribute component=control`
#[derive(Debug, Clone, Encode, Decode, CborLen, PartialEq, Eq)]
#[rustfmt::skip]
#[cbor(map)]
pub struct OneTimeCode {
    #[n(1)] code: ByteArray<32>,
}

impl Display for OidcToken {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.access_token.0)
    }
}

impl OneTimeCode {
    /// Create a random token
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let mut code = [0; 32];
        rand::thread_rng().fill_bytes(&mut code);
        OneTimeCode::from(code)
    }

    /// Return the code as a byte slice
    pub fn code(&self) -> &[u8; 32] {
        &self.code
    }
}

impl From<[u8; 32]> for OneTimeCode {
    /// Create a OneTimeCode from a byte slice
    fn from(code: [u8; 32]) -> Self {
        OneTimeCode { code: code.into() }
    }
}

impl FromStr for OneTimeCode {
    type Err = Error;

    /// Create a OneTimeCode from a string slice
    /// The code is expected to be encoded as hexadecimal
    fn from_str(s: &str) -> Result<Self> {
        let bytes = hex::decode(s).map_err(|e| error(format!("{e}")))?;
        let code: OneTimeCode = OneTimeCode::from(
            <[u8; 32]>::try_from(bytes.as_slice()).map_err(|e| error(format!("{e}")))?,
        );
        Ok(code)
    }
}

impl ToString for OneTimeCode {
    /// Return the OneTimeCode as a String
    /// It is encoded as hexadecimal
    fn to_string(&self) -> String {
        hex::encode(self.code())
    }
}

impl Serialize for OneTimeCode {
    fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.to_string().as_str())
    }
}

impl<'de> Deserialize<'de> for OneTimeCode {
    fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        OneTimeCode::from_str(s.as_str()).map_err(serde::de::Error::custom)
    }
}

impl ToSqlxType for OneTimeCode {
    fn to_sql(&self) -> SqlxType {
        self.to_string().to_sql()
    }
}

/// Create an Identity Error
fn error(message: String) -> Error {
    Error::new(Origin::Identity, Kind::Invalid, message.as_str())
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen};
    use quickcheck_macros::quickcheck;

    #[quickcheck]
    fn test_from_to_string(one_time_code: OneTimeCode) -> bool {
        OneTimeCode::from_str(one_time_code.to_string().as_str()).ok() == Some(one_time_code)
    }

    impl Arbitrary for OneTimeCode {
        fn arbitrary(g: &mut Gen) -> Self {
            OneTimeCode::from(Bytes32::arbitrary(g).bytes)
        }
    }

    /// Newtype to generate an arbitrary array of 32 bytes
    /// This can be refactored into a ockam_quickcheck crate if we accumulate
    /// more useful arbitraries which can be shared by several crates
    #[derive(Clone)]
    struct Bytes32 {
        bytes: [u8; 32],
    }

    impl Arbitrary for Bytes32 {
        fn arbitrary(g: &mut Gen) -> Bytes32 {
            let init: [u8; 32] = <[u8; 32]>::default();
            Bytes32 {
                bytes: init.map(|_| <u8>::arbitrary(g)),
            }
        }

        /// there is no meaningful shrinking in general for a random array of bytes
        fn shrink(&self) -> Box<dyn Iterator<Item = Bytes32>> {
            Box::new(std::iter::empty())
        }
    }
}
