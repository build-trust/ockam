use serde::{Deserialize, Serialize};
use serde_big_array::big_array;

big_array! { BigArray; }

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Eq, PartialEq)]
pub enum SignatureType {
    RootSign,
}

/// Variants of proofs that are allowed on a [`Profile`] change
#[derive(Clone, Debug)]
pub enum ProfileChangeProof {
    Signature(Signature),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Signature {
    stype: SignatureType,
    #[serde(with = "BigArray")]
    data: [u8; 64],
}

impl Signature {
    pub fn stype(&self) -> &SignatureType {
        &self.stype
    }
    pub fn data(&self) -> &[u8; 64] {
        &self.data
    }
}

impl Signature {
    pub fn new(stype: SignatureType, data: [u8; 64]) -> Self {
        Signature { stype, data }
    }
}
