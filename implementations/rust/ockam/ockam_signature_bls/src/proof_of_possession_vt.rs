use crate::{PublicKeyVt, SecretKey};
use bls12_381_plus::{
    multi_miller_loop, ExpandMsgXmd, G1Affine, G2Affine, G2Prepared, G2Projective,
};
use core::ops::Neg;
use ff::Field;
use group::{Curve, Group};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use subtle::{Choice, CtOption};

/// A proof of possession of the secret key
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProofOfPossessionVt(pub(crate) G2Projective);

impl Default for ProofOfPossessionVt {
    fn default() -> Self {
        Self(G2Projective::identity())
    }
}

impl Serialize for ProofOfPossessionVt {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(s)
    }
}

impl<'de> Deserialize<'de> for ProofOfPossessionVt {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let p = G2Projective::deserialize(d)?;
        Ok(Self(p))
    }
}

impl ProofOfPossessionVt {
    /// Number of bytes needed to represent the proof
    pub const BYTES: usize = 96;
    /// The domain separation tag
    const DST: &'static [u8] = b"BLS_POP_BLS12381G2_XMD:SHA-256_SSWU_RO_POP_";

    /// Create a new proof of possession
    pub fn new(sk: &SecretKey) -> Option<Self> {
        if sk.0.is_zero() {
            return None;
        }
        let pk = PublicKeyVt::from(sk);
        let a = G2Projective::hash::<ExpandMsgXmd<sha2::Sha256>>(&pk.to_bytes(), Self::DST);
        Some(Self(a * sk.0))
    }

    /// Verify if the proof is over `pk`
    pub fn verify(&self, pk: PublicKeyVt) -> Choice {
        if pk.0.is_identity().unwrap_u8() == 1 {
            return Choice::from(0);
        }
        let a = G2Projective::hash::<ExpandMsgXmd<sha2::Sha256>>(&pk.to_bytes(), Self::DST);
        let g1 = G1Affine::generator().neg();

        multi_miller_loop(&[
            (&pk.0.to_affine(), &G2Prepared::from(a.to_affine())),
            (&g1, &G2Prepared::from(self.0.to_affine())),
        ])
        .final_exponentiation()
        .is_identity()
    }

    /// Get the byte sequence that represents this proof
    pub fn to_bytes(&self) -> [u8; Self::BYTES] {
        self.0.to_affine().to_compressed()
    }

    /// Convert a big-endian representation of the proof
    pub fn from_bytes(bytes: &[u8; Self::BYTES]) -> CtOption<Self> {
        let mut t = [0u8; Self::BYTES];
        t.copy_from_slice(bytes);
        G2Affine::from_compressed(&t).map(|p| Self(G2Projective::from(&p)))
    }
}

#[test]
fn pop_vt_works() {
    use crate::MockRng;
    use rand_core::SeedableRng;

    let seed = [2u8; 16];
    let mut rng = MockRng::from_seed(seed);
    let sk = SecretKey::random(&mut rng).unwrap();
    let pop = ProofOfPossessionVt::new(&sk).unwrap();
    let pk = PublicKeyVt::from(&sk);
    assert_eq!(pop.verify(pk).unwrap_u8(), 1);
}
