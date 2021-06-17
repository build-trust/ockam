use bls12_381_plus::{G1Affine, G1Projective};
use group::Curve;
use serde::{Deserialize, Serialize};
use subtle::CtOption;

/// Represents one or more commitments as
/// x * G1 + ...
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Commitment(pub G1Projective);

impl Commitment {
    /// Number of bytes needed to represent the commitment
    pub const BYTES: usize = 48;

    /// Get the byte sequence that represents this signature
    pub fn to_bytes(&self) -> [u8; Self::BYTES] {
        self.0.to_affine().to_compressed()
    }

    /// Convert a big-endian representation of the commitment
    pub fn from_bytes(bytes: &[u8; Self::BYTES]) -> CtOption<Self> {
        G1Affine::from_compressed(bytes).map(|p| Self(G1Projective::from(&p)))
    }
}

#[cfg(test)]
mod test {
    use crate::commitment::Commitment;
    use bls12_381_plus::G1Affine;

    #[test]
    fn test_commitment() {
        let g1 = G1Affine::generator().to_compressed();
        let c = Commitment::from_bytes(&g1).unwrap();
        let cb = c.to_bytes();
        let co = Commitment::from_bytes(&cb).unwrap();
        assert_eq!(c, co);

        let json = serde_json::to_string(&co).unwrap();
        assert_eq!("[151,241,211,167,49,151,215,148,38,149,99,140,79,169,172,15,195,104,140,79,151,116,185,5,161,78,58,63,23,27,172,88,108,85,232,63,249,122,26,239,251,58,240,10,219,34,198,187]", json);
    }
}
