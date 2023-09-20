use ockam_core::compat::{string::String, vec::Vec};
use ockam_core::env::FromString;
use ockam_core::{Error, Result};

use crate::{Identifier, IdentityError};

use crate::models::{ChangeHash, CHANGE_HASH_LEN, IDENTIFIER_LEN};
use core::fmt::{Display, Formatter};
use core::str::FromStr;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

impl Serialize for Identifier {
    fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&String::from(self))
    }
}

impl<'de> Deserialize<'de> for Identifier {
    fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let str: String = Deserialize::deserialize(deserializer)?;

        Self::try_from(str).map_err(de::Error::custom)
    }
}

impl Identifier {
    const PREFIX: &'static str = "I";
}

impl Display for Identifier {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.write_str(&String::from(self))
    }
}

impl From<Identifier> for String {
    fn from(id: Identifier) -> Self {
        String::from(&id)
    }
}

impl From<&Identifier> for String {
    fn from(id: &Identifier) -> Self {
        format!("{}{}", Identifier::PREFIX, hex::encode(id.0.as_ref()))
    }
}

impl TryFrom<&str> for Identifier {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self> {
        let value = value.trim();
        if let Some(value) = value.strip_prefix(Self::PREFIX) {
            if let Ok(data) = hex::decode(value) {
                data.try_into()
            } else {
                Err(IdentityError::InvalidIdentifier.into())
            }
        } else {
            Err(IdentityError::InvalidIdentifier.into())
        }
    }
}

impl TryFrom<&[u8]> for Identifier {
    type Error = Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if let Ok(value) = <[u8; IDENTIFIER_LEN]>::try_from(value) {
            Ok(Self(value))
        } else {
            Err(IdentityError::InvalidIdentifier.into())
        }
    }
}

impl TryFrom<Vec<u8>> for Identifier {
    type Error = Error;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        Self::try_from(value.as_slice())
    }
}

impl TryFrom<String> for Identifier {
    type Error = Error;

    fn try_from(value: String) -> Result<Self> {
        Self::try_from(value.as_str())
    }
}

impl FromStr for Identifier {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.try_into()
    }
}

impl FromString for Identifier {
    fn from_string(s: &str) -> Result<Self> {
        s.try_into()
    }
}

impl From<ChangeHash> for Identifier {
    fn from(value: ChangeHash) -> Self {
        Self(value.0)
    }
}

impl Display for ChangeHash {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.write_str(&String::from(self))
    }
}

impl From<ChangeHash> for String {
    fn from(change_hash: ChangeHash) -> Self {
        String::from(&change_hash)
    }
}

impl From<&ChangeHash> for String {
    fn from(change_hash: &ChangeHash) -> Self {
        hex::encode(change_hash.0.as_ref())
    }
}

impl TryFrom<&str> for ChangeHash {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self> {
        let value = value.trim();
        if let Ok(data) = hex::decode(value) {
            data.try_into()
        } else {
            Err(IdentityError::InvalidIdentifier.into())
        }
    }
}

impl TryFrom<&[u8]> for ChangeHash {
    type Error = Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if let Ok(value) = <[u8; CHANGE_HASH_LEN]>::try_from(value) {
            Ok(Self(value))
        } else {
            Err(IdentityError::InvalidIdentifier.into())
        }
    }
}

impl TryFrom<Vec<u8>> for ChangeHash {
    type Error = Error;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        Self::try_from(value.as_slice())
    }
}

impl TryFrom<String> for ChangeHash {
    type Error = Error;

    fn try_from(value: String) -> Result<Self> {
        Self::try_from(value.as_str())
    }
}

impl FromStr for ChangeHash {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.try_into()
    }
}

impl FromString for ChangeHash {
    fn from_string(s: &str) -> Result<Self> {
        s.try_into()
    }
}

impl AsRef<[u8]> for ChangeHash {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::{quickcheck, Arbitrary, Gen};

    impl Arbitrary for Identifier {
        fn arbitrary(g: &mut Gen) -> Self {
            let mut data = [0u8; IDENTIFIER_LEN];
            for i in data.iter_mut() {
                *i = u8::arbitrary(g);
            }

            Self(data)
        }
    }

    quickcheck! {
        fn check_from_string(id: Identifier) -> bool {
            id == Identifier::try_from(id.to_string()).unwrap()
        }

        fn check_from_str(id: Identifier) -> bool {
            id == Identifier::try_from(id.to_string().as_str()).unwrap()
        }

        fn check_from_slice(id: Identifier) -> bool {
            id == Identifier::try_from(id.0.as_slice()).unwrap()
        }

        fn check_from_vec(id: Identifier) -> bool {
            id == Identifier::try_from(id.0.to_vec()).unwrap()
        }

        fn check_encode_decode(id: Identifier) -> bool {
            id == minicbor::decode(&minicbor::to_vec(&id).unwrap()).unwrap()
        }

        fn check_serialize_deserialize(id: Identifier) -> bool {
            id == serde_bare::from_slice(&serde_bare::to_vec(&id).unwrap()).unwrap()
        }

        fn prop_prefix(id: Identifier) -> bool {
            id.to_string().starts_with(Identifier::PREFIX)
        }
    }
}
