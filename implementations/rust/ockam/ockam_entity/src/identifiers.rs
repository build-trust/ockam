use crate::profile::Profile;
use ockam_core::hex::encode;
use ockam_core::Result;
use ockam_vault_core::{Hasher, KeyId};
use serde::{Deserialize, Serialize};

/// An identifier of a Profile.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize, Default)]
pub struct ProfileIdentifier(KeyId);

/// Unique [`crate::Profile`] identifier, computed as SHA256 of root public key
impl ProfileIdentifier {
    /// Create a ProfileIdentifier from a KeyId
    pub fn from_key_id(key_id: KeyId) -> Self {
        Self { 0: key_id }
    }
    /// Human-readable form of the id
    pub fn to_external(&self) -> String {
        format!("P_ID.{}", &self.0)
    }
    pub fn from_external(str: &str) -> Result<Self> {
        let str = str.strip_prefix("P_ID.").unwrap(); // FIXME
        Ok(Self::from(str))
    }

    /// Return the wrapped KeyId
    pub fn key_id(&self) -> &KeyId {
        &self.0
    }
}

impl From<&str> for ProfileIdentifier {
    fn from(s: &str) -> Self {
        ProfileIdentifier::from_key_id(s.into())
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
    use rand::{thread_rng, RngCore};

    impl ProfileIdentifier {
        pub fn random() -> ProfileIdentifier {
            ProfileIdentifier(format!("{:x}", thread_rng().next_u64()))
        }
    }

    #[test]
    fn test_new() {
        let _identifier = ProfileIdentifier::from_key_id("test".to_string());
    }
}
