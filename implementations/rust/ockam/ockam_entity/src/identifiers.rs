use crate::profile::Profile;
use crate::EntityError;
use core::convert::TryFrom;
use core::fmt::{Display, Formatter};
use ockam_core::compat::string::String;
use ockam_core::hex::encode;
use ockam_core::{Error, Result};
use ockam_vault_core::{Hasher, KeyId};
use serde::{Deserialize, Serialize};

/// An identifier of a Profile.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize, Default)]
pub struct EntityIdentifier(KeyId);

pub type ProfileIdentifier = EntityIdentifier;

/// Unique [`crate::Profile`] identifier, computed as SHA256 of root public key
impl EntityIdentifier {
    pub const PREFIX: &'static str = "P";
    /// Create a EntityIdentifier from a KeyId
    pub fn from_key_id(key_id: KeyId) -> Self {
        Self { 0: key_id }
    }
    /// Return the wrapped KeyId
    pub fn key_id(&self) -> &KeyId {
        &self.0
    }
}

impl Display for EntityIdentifier {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        let str: String = self.clone().into();
        write!(f, "{}", &str)
    }
}

impl Into<String> for EntityIdentifier {
    fn into(self) -> String {
        format!("{}{}", Self::PREFIX, &self.0)
    }
}

impl TryFrom<&str> for EntityIdentifier {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self> {
        if let Some(str) = value.strip_prefix(Self::PREFIX) {
            Ok(Self::from_key_id(str.into()))
        } else {
            Err(EntityError::InvalidProfileId.into())
        }
    }
}

impl TryFrom<String> for EntityIdentifier {
    type Error = Error;

    fn try_from(value: String) -> Result<Self> {
        Self::try_from(value.as_str())
    }
}

/// Unique [`crate::ProfileChangeEvent`] identifier, computed as SHA256 of the event data
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash)]
pub struct EventIdentifier([u8; 32]);

impl AsRef<[u8]> for EventIdentifier {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl EventIdentifier {
    pub fn initial<H: Hasher>(mut hasher: H) -> Self {
        let h = match hasher.sha256(Profile::NO_EVENT) {
            Ok(hash) => hash,
            Err(_) => panic!("failed to hash initial event"),
        };
        EventIdentifier::from_hash(h)
    }
    pub async fn async_initial<H: Hasher>(mut hasher: H) -> Self {
        let h = match hasher.async_sha256(Profile::NO_EVENT).await {
            Ok(hash) => hash,
            Err(_) => panic!("failed to hash initial event"),
        };
        EventIdentifier::from_hash(h)
    }
    /// Create identifier from public key hash
    pub fn from_hash(hash: [u8; 32]) -> Self {
        Self { 0: hash }
    }
    /// Human-readable form of the id
    pub fn to_string_representation(&self) -> String {
        format!("E_ID.{}", encode(&self.0))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use core::convert::TryInto;
    use rand::{thread_rng, RngCore};

    impl EntityIdentifier {
        pub fn random() -> EntityIdentifier {
            EntityIdentifier(format!("{:x}", thread_rng().next_u64()))
        }
    }

    #[test]
    fn test_new() {
        let _identifier = EntityIdentifier::from_key_id("test".to_string());
    }

    #[test]
    fn test_into() {
        let id1 = EntityIdentifier::random();

        let str: String = id1.clone().into();
        assert!(str.starts_with("P"));

        let id2: EntityIdentifier = str.try_into().unwrap();
        assert_eq!(id1, id2);
    }
}
