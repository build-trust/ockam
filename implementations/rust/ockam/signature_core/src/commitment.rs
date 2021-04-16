use bls12_381_plus::{G1Affine, G1Projective};
use group::Curve;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use subtle::CtOption;

/// Represents one or more commitments as
/// x * G1 + ...
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Commitment(pub G1Projective);

impl Serialize for Commitment {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(s)
    }
}

impl<'de> Deserialize<'de> for Commitment {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let p = G1Projective::deserialize(d)?;
        Ok(Self(p))
    }
}

impl Commitment {
    /// Number of bytes needed to represent the commitment
    pub const BYTES: usize = 48;

    /// Get the byte sequence that represents this signature
    pub fn to_bytes(&self) -> [u8; Self::BYTES] {
        self.0.to_affine().to_compressed()
    }

    /// Convert a big-endian representation of the commitment
    pub fn from_bytes(bytes: &[u8; Self::BYTES]) -> CtOption<Self> {
        G1Affine::from_compressed(&bytes).map(|p| Self(G1Projective::from(&p)))
    }
}
