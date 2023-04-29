use core::fmt::{Display, Formatter};
use ockam_core::compat::string::String;
use serde::{Deserialize, Serialize};

/// Unique [`crate::change::IdentityChange`] identifier, computed as SHA256 of the change data
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash)]
pub struct ChangeIdentifier([u8; 32]);

impl Display for ChangeIdentifier {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

impl AsRef<[u8]> for ChangeIdentifier {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl ChangeIdentifier {
    /// Create identifier from public key hash
    pub fn from_hash(hash: [u8; 32]) -> Self {
        Self(hash)
    }
    /// Human-readable form of the id
    pub fn to_string_representation(&self) -> String {
        format!("E_ID.{}", hex::encode(self.0))
    }
}
