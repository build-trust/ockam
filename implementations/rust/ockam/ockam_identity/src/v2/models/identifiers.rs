use core::fmt::{Display, Formatter};
use core::ops::Deref;
use core::str::FromStr;
use minicbor::bytes::ByteArray;
use minicbor::encode::Write;
use minicbor::{Decode, Decoder, Encode, Encoder};
use ockam_core::compat::{string::String, vec::Vec};
use ockam_core::env::FromString;
use ockam_core::{Error, Result};
use serde::{Deserialize, Serialize};

use super::super::IdentityError;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Identifier([u8; 20]);

impl<C> Encode<C> for Identifier {
    fn encode<W: Write>(
        &self,
        e: &mut Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        ByteArray::from(self.0).encode(e, ctx)
    }
}

impl<'b, C> Decode<'b, C> for Identifier {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let data = ByteArray::<20>::decode(d, ctx)?;

        Ok(Self(*data.deref()))
    }
}

impl Identifier {
    const PREFIX: &'static str = "I";

    pub(crate) fn ct_eq(&self, o: &Self) -> subtle::Choice {
        use subtle::ConstantTimeEq;
        self.0.as_ref().ct_eq(o.0.as_ref())
    }
}

impl Eq for Identifier {}

impl PartialEq for Identifier {
    fn eq(&self, o: &Self) -> bool {
        self.ct_eq(o).into()
    }
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
        if let Ok(value) = <[u8; 20]>::try_from(value) {
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

// sha256 hash of CBOR serialized Change
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ChangeHash([u8; 20]);

impl<C> Encode<C> for ChangeHash {
    fn encode<W: Write>(
        &self,
        e: &mut Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        ByteArray::from(self.0).encode(e, ctx)
    }
}

impl<'b, C> Decode<'b, C> for ChangeHash {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let data = ByteArray::<20>::decode(d, ctx)?;

        Ok(Self(*data.deref()))
    }
}

impl ChangeHash {
    pub(crate) fn new(hash: [u8; 20]) -> Self {
        Self(hash)
    }

    fn ct_eq(&self, o: &Self) -> subtle::Choice {
        use subtle::ConstantTimeEq;
        self.0.as_ref().ct_eq(o.0.as_ref())
    }
}

impl Eq for ChangeHash {}

impl PartialEq for ChangeHash {
    fn eq(&self, o: &Self) -> bool {
        self.ct_eq(o).into()
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
        if let Ok(value) = <[u8; 20]>::try_from(value) {
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
