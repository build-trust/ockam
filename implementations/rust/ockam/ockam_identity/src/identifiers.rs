use crate::{IdentityError, IdentityStateConst};
use core::fmt::{Display, Formatter};
use ockam_core::compat::string::String;
use ockam_core::vault::{Hasher, KeyId};
use ockam_core::{Error, Result};
use serde::{Deserialize, Serialize};

/// An identifier of an Identity.
#[allow(clippy::derive_hash_xor_eq)] // we manually implement a constant time Eq
#[derive(Clone, Debug, Hash, Serialize, Deserialize, Default, PartialOrd, Ord)]
pub struct IdentityIdentifier(KeyId);

/// Unique [`crate::Identity`] identifier, computed as SHA256 of root public key
impl IdentityIdentifier {
    pub const PREFIX: &'static str = "P";
    /// Create an IdentityIdentifier from a KeyId
    pub fn from_key_id(key_id: KeyId) -> Self {
        Self(key_id)
    }
    /// Return the wrapped KeyId
    pub fn key_id(&self) -> &KeyId {
        &self.0
    }
    pub(crate) fn ct_eq(&self, o: &Self) -> subtle::Choice {
        use subtle::ConstantTimeEq;
        self.0.as_bytes().ct_eq(o.0.as_bytes())
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
        let str: String = self.clone().into();
        write!(f, "{}", &str)
    }
}

impl From<IdentityIdentifier> for String {
    fn from(id: IdentityIdentifier) -> Self {
        format!("{}{}", IdentityIdentifier::PREFIX, &id.0)
    }
}

impl TryFrom<&str> for IdentityIdentifier {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self> {
        if let Some(str) = value.strip_prefix(Self::PREFIX) {
            Ok(Self::from_key_id(str.into()))
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
    use super::*;
    use rand::{thread_rng, RngCore};

    impl IdentityIdentifier {
        pub fn random() -> IdentityIdentifier {
            IdentityIdentifier(format!("{:x}", thread_rng().next_u64()))
        }
    }

    #[test]
    fn test_new() {
        let _identifier = IdentityIdentifier::from_key_id("test".to_string());
    }

    #[test]
    fn test_into() {
        let id1 = IdentityIdentifier::random();

        let str: String = id1.clone().into();
        assert!(str.starts_with('P'));

        let id2: IdentityIdentifier = str.try_into().unwrap();
        assert_eq!(id1, id2);
    }
}
