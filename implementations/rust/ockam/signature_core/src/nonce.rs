use crate::util::*;
use bls12_381_plus::Scalar;
use ff::Field;
use rand_core::RngCore;
use serde::{Deserialize, Serialize};
use subtle::CtOption;

/// A nonce that is used for zero-knowledge proofs
#[derive(Debug, Copy, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct Nonce(pub Scalar);

impl Nonce {
    /// The number of bytes in a nonce
    pub const BYTES: usize = 32;

    /// Hash arbitrary data to a nonce
    pub fn hash<B: AsRef<[u8]>>(data: B) -> Self {
        Self(hash_to_scalar(data))
    }

    /// Generate a random nonce
    pub fn random(rng: impl RngCore) -> Self {
        Self(Scalar::random(rng))
    }

    /// Get the byte sequence that represents this nonce
    pub fn to_bytes(&self) -> [u8; Self::BYTES] {
        scalar_to_bytes(self.0)
    }

    /// Convert a big-endian representation of the nonce
    pub fn from_bytes(bytes: &[u8; Self::BYTES]) -> CtOption<Self> {
        scalar_from_bytes(bytes).map(Self)
    }
}

#[cfg(test)]
mod test {
    use crate::lib::Nonce;
    use rand::thread_rng;

    #[test]
    fn test_nonce() {
        let h = [0_u8; 32];
        let n = Nonce::hash(h);
        let nr = Nonce::random(thread_rng());
        assert_ne!(n, nr);

        let nb = n.to_bytes();
        let n2 = Nonce::from_bytes(&nb).unwrap();
        assert_eq!(n, n2);
    }
}
