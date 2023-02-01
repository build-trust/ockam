use core::fmt;
use ockam_core::vault::Signature;
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
pub struct IdentityChangeSignature {
    stype: SignatureType,
    data: Signature,
}

impl IdentityChangeSignature {
    /// Return the signature type
    pub fn stype(&self) -> &SignatureType {
        &self.stype
    }
    /// Return signature data
    pub fn data(&self) -> &Signature {
        &self.data
    }
}

impl IdentityChangeSignature {
    /// Create a new signature
    pub fn new(stype: SignatureType, data: Signature) -> Self {
        IdentityChangeSignature { stype, data }
    }
}

impl fmt::Display for IdentityChangeSignature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} {}", self.stype(), hex::encode(self.data()))
    }
}
