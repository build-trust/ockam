use crate::identity::identity_change::ChangeIdentifier;
use crate::identity::identity_change::KeyAttributes;
use core::fmt;
use ockam_core::vault::PublicKey;
use serde::{Deserialize, Serialize};

/// RotateKeyChangeData
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RotateKeyChangeData {
    prev_change_id: ChangeIdentifier,
    key_attributes: KeyAttributes,
    public_key: PublicKey,
}

impl RotateKeyChangeData {
    /// Return key attributes
    pub fn key_attributes(&self) -> &KeyAttributes {
        &self.key_attributes
    }
    /// Return public key
    pub fn public_key(&self) -> &PublicKey {
        &self.public_key
    }
    /// Previous change identifier, used to create a chain
    pub fn prev_change_id(&self) -> &ChangeIdentifier {
        &self.prev_change_id
    }
}

impl RotateKeyChangeData {
    /// Create RotateKeyChangeData
    pub fn new(
        prev_change_id: ChangeIdentifier,
        key_attributes: KeyAttributes,
        public_key: PublicKey,
    ) -> Self {
        Self {
            prev_change_id,
            key_attributes,
            public_key,
        }
    }
}

impl fmt::Display for RotateKeyChangeData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "prev_change_id:{} key attibutes:{} public key:{}",
            self.prev_change_id(),
            self.key_attributes(),
            self.public_key()
        )
    }
}
