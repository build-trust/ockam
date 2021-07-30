use ockam_vault::ockam_vault_core::Signature as OckamVaultSignature;
use serde::{Deserialize, Serialize};

/// Types of proof signatures.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Eq, PartialEq)]
pub enum SignatureType {
    /// Root signature
    RootSign,
}

/// Variants of proofs that are allowed on a [`crate::Profile`] change
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ProfileChangeProof {
    /// Signature change proof
    Signature(Signature),
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
