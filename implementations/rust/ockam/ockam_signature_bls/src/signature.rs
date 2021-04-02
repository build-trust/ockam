use crate::{PublicKey, SecretKey};
use bls12_381_plus::{
    multi_miller_loop, ExpandMsgXmd, G1Affine, G1Projective, G2Affine, G2Prepared,
};
use core::ops::{BitOr, Neg, Not};
use ff::Field;
use group::{Curve, Group};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use subtle::{Choice, CtOption};

/// Represents a BLS signature in G1 using the proof of possession scheme
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Signature(pub(crate) G1Projective);

impl Default for Signature {
    fn default() -> Self {
        Self(G1Projective::identity())
    }
}

impl Serialize for Signature {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(s)
    }
}

impl<'de> Deserialize<'de> for Signature {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let p = G1Projective::deserialize(d)?;
        Ok(Self(p))
    }
}

impl Signature {
    /// Number of bytes needed to represent the signature
    pub const BYTES: usize = 48;
    /// The domain separation tag
    const DST: &'static [u8] = b"BLS_SIG_BLS12381G1_XMD:SHA-256_SSWU_RO_POP_";

    /// Create a new bls
    pub fn new<B: AsRef<[u8]>>(sk: &SecretKey, msg: B) -> Option<Self> {
        if sk.0.is_zero() {
            return None;
        }
        let a = Self::hash_msg(msg.as_ref());
        Some(Self(a * sk.0))
    }

    pub(crate) fn hash_msg(msg: &[u8]) -> G1Projective {
        G1Projective::hash::<ExpandMsgXmd<sha2::Sha256>>(msg, Self::DST)
    }

    /// Check if this signature is valid
    pub fn is_valid(&self) -> Choice {
        self.0.is_identity().not().bitor(self.0.is_on_curve())
    }

    /// Check if this signature is invalid
    pub fn is_invalid(&self) -> Choice {
        self.0.is_identity().bitor(self.0.is_on_curve().not())
    }

    /// Verify if the bls is over `msg` with `pk`
    pub fn verify<B: AsRef<[u8]>>(&self, pk: PublicKey, msg: B) -> Choice {
        if pk.0.is_identity().bitor(self.is_invalid()).unwrap_u8() == 1 {
            return Choice::from(0);
        }
        let a = Self::hash_msg(msg.as_ref());
        let g2 = G2Affine::generator().neg();

        multi_miller_loop(&[
            (&a.to_affine(), &G2Prepared::from(pk.0.to_affine())),
            (&self.0.to_affine(), &G2Prepared::from(g2)),
        ])
        .final_exponentiation()
        .is_identity()
    }

    /// Get the byte sequence that represents this signature
    pub fn to_bytes(&self) -> [u8; Self::BYTES] {
        self.0.to_affine().to_compressed()
    }

    /// Convert a big-endian representation of the signature
    pub fn from_bytes(bytes: &[u8; Self::BYTES]) -> CtOption<Self> {
        G1Affine::from_compressed(&bytes).map(|p| Self(G1Projective::from(&p)))
    }
}

#[test]
fn signature_works() {
    use crate::MockRng;
    use rand_core::{RngCore, SeedableRng};

    let seed = [2u8; 16];
    let mut rng = MockRng::from_seed(seed);
    let sk = SecretKey::random(&mut rng).unwrap();
    let mut msg = [0u8; 12];
    rng.fill_bytes(&mut msg);
    let sig = Signature::new(&sk, msg).unwrap();
    let pk = PublicKey::from(&sk);
    assert_eq!(sig.verify(pk, msg).unwrap_u8(), 1);
}
