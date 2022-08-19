use crate::{IdentityError, IdentityStateConst};
use core::fmt::{Display, Formatter};
use core::str::FromStr;
use minicbor::decode::{self, Decoder};
use minicbor::{Decode, Encode};
use ockam_core::compat::borrow::Cow;
use ockam_core::compat::string::{String, ToString};
use ockam_core::vault::{Hasher, KeyId};
use ockam_core::{Error, Result};
use serde::{Deserialize, Deserializer, Serialize};

/// An identifier of an Identity.
#[allow(clippy::derive_hash_xor_eq)] // we manually implement a constant time Eq
#[derive(Clone, Debug, Hash, Encode, Serialize, Default, PartialOrd, Ord)]
#[cbor(transparent)]
pub struct IdentityIdentifier(#[n(0)] KeyId);

/// Unique [`crate::Identity`] identifier, computed as SHA256 of root public key
impl IdentityIdentifier {
    const PREFIX: &'static str = "P";

    /// Create an IdentityIdentifier from a KeyId
    pub fn from_key_id(key_id: &str) -> Self {
        Self(format!("{}{}", Self::PREFIX, key_id.trim()))
    }

    /// Return the wrapped KeyId
    pub fn key_id(&self) -> &str {
        &self.0[Self::PREFIX.len()..]
    }

    pub(crate) fn ct_eq(&self, o: &Self) -> subtle::Choice {
        use subtle::ConstantTimeEq;
        self.0.as_bytes().ct_eq(o.0.as_bytes())
    }
}

impl<'b, C> Decode<'b, C> for IdentityIdentifier {
    fn decode(d: &mut Decoder<'b>, _: &mut C) -> Result<Self, decode::Error> {
        d.str()?.try_into().map_err(decode::Error::message)
    }
}

impl<'de> Deserialize<'de> for IdentityIdentifier {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        <Cow<'de, str>>::deserialize(d)?
            .as_ref()
            .try_into()
            .map_err(serde::de::Error::custom)
    }
}

impl Eq for IdentityIdentifier {}

impl PartialEq for IdentityIdentifier {
    fn eq(&self, o: &Self) -> bool {
        self.ct_eq(o).into()
    }
}

impl Display for IdentityIdentifier {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        self.serialize(f)
    }
}

impl From<IdentityIdentifier> for String {
    fn from(id: IdentityIdentifier) -> Self {
        id.0
    }
}

impl From<&IdentityIdentifier> for String {
    fn from(id: &IdentityIdentifier) -> Self {
        id.0.to_string()
    }
}

impl TryFrom<&str> for IdentityIdentifier {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self> {
        let value = value.trim();
        if value.starts_with(Self::PREFIX) {
            Ok(Self(value.to_string()))
        } else {
            Err(IdentityError::InvalidIdentityId.into())
        }
    }
}

impl TryFrom<String> for IdentityIdentifier {
    type Error = Error;

    fn try_from(value: String) -> Result<Self> {
        Self::try_from(value.as_str())
    }
}

impl FromStr for IdentityIdentifier {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.try_into()
    }
}

/// Unique [`crate::IdentityChangeEvent`] identifier, computed as SHA256 of the event data
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash)]
pub struct EventIdentifier([u8; 32]);

impl AsRef<[u8]> for EventIdentifier {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl EventIdentifier {
    pub async fn initial(hasher: &(impl Hasher + Sync)) -> Self {
        let h = match hasher.sha256(IdentityStateConst::NO_EVENT).await {
            Ok(hash) => hash,
            Err(_) => panic!("failed to hash initial event"),
        };
        EventIdentifier::from_hash(h)
    }
    /// Create identifier from public key hash
    pub fn from_hash(hash: [u8; 32]) -> Self {
        Self(hash)
    }
    /// Human-readable form of the id
    pub fn to_string_representation(&self) -> String {
        format!("E_ID.{}", hex::encode(&self.0))
    }
}

#[cfg(test)]
mod test {
    use super::IdentityIdentifier;
    use quickcheck::{quickcheck, Arbitrary, Gen};
    use serde::de::{value, Deserialize, IntoDeserializer};

    #[derive(Debug, Clone)]
    struct Id(IdentityIdentifier);

    impl Arbitrary for Id {
        fn arbitrary(g: &mut Gen) -> Self {
            Self(IdentityIdentifier::from_key_id(&String::arbitrary(g)))
        }
    }

    impl IdentityIdentifier {
        pub fn random() -> IdentityIdentifier {
            Id::arbitrary(&mut Gen::new(32)).0
        }
    }

    quickcheck! {
        fn prop_to_str_from_str(val: Id) -> bool {
            let s = val.0.to_string();
            val.0 == IdentityIdentifier::try_from(s).unwrap()
        }

        fn prop_encode_decode(val: Id) -> bool {
            let b = minicbor::to_vec(&val.0).unwrap();
            let i = minicbor::decode(&b).unwrap();
            val.0 == i
        }

        fn prop_serialize_deserialize(val: Id) -> bool {
            let s = val.0.to_string();
            let d = IntoDeserializer::<value::Error>::into_deserializer(s);
            let i = IdentityIdentifier::deserialize(d).unwrap();
            val.0 == i
        }

        fn prop_eq_key_id(s: String) -> bool {
            let k = s.trim();
            let i = IdentityIdentifier::from_key_id(k);
            i.key_id() == k
        }

        fn prop_prefix(val: Id) -> bool {
            val.0.0.starts_with(IdentityIdentifier::PREFIX)
        }
    }
}
