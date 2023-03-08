use core::fmt;
use ockam_core::vault::Signature as OckamVaultSignature;
use serde::{Deserialize, Serialize};

/// Types of proof signatures.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Eq, PartialEq)]
pub enum SignatureType {
    /// Root signature
    RootSign,
    /// Self signature
    SelfSign,
    /// Signature using previous key
    PrevSign,
}

/// Signature, its type and data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Signature {
    stype: SignatureType,
    data: OckamVaultSignature,
}

impl Signature {
    /// Return the signature type
    pub fn stype(&self) -> &SignatureType {
        &self.stype
    }
    /// Return signature data
    pub fn data(&self) -> &OckamVaultSignature {
        &self.data
    }
}

impl Signature {
    /// Create a new signature
    pub fn new(stype: SignatureType, data: OckamVaultSignature) -> Self {
        Signature { stype, data }
    }
}

impl fmt::Display for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} {}", self.stype(), hex::encode(self.data()))
    }
}
