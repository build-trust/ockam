use crate::partial_signature::PARTIAL_SIGNATURE_BYTES;
use crate::{PartialSignature, PublicKey, SecretKey};
use bls12_381_plus::{
    multi_miller_loop, ExpandMsgXmd, G1Affine, G1Projective, G2Affine, G2Prepared, Scalar,
};
use core::{
    fmt::{self, Display},
    ops::{BitOr, Neg, Not},
};
use ff::Field;
use group::{Curve, Group};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use subtle::{Choice, CtOption};
use vsss_rs::{Error, Shamir, Share};

/// Represents a BLS signature in G1 using the proof of possession scheme
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Signature(pub(crate) G1Projective);

impl Display for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

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
    pub fn to_bytes(self) -> [u8; Self::BYTES] {
        self.0.to_affine().to_compressed()
    }

    /// Convert a big-endian representation of the signature
    pub fn from_bytes(bytes: &[u8; Self::BYTES]) -> CtOption<Self> {
        G1Affine::from_compressed(bytes).map(|p| Self(G1Projective::from(&p)))
    }

    /// Combine partial signatures into a completed signature
    pub fn from_partials<const T: usize, const N: usize>(
        partials: &[PartialSignature],
    ) -> Result<Self, Error> {
        if T > partials.len() {
            return Err(Error::SharingLimitLessThanThreshold);
        }
        let mut pp = [Share::<PARTIAL_SIGNATURE_BYTES>::default(); T];
        for i in 0..T {
            pp[i] = partials[i].0;
        }
        let point =
            Shamir::<T, N>::combine_shares_group::<Scalar, G1Projective, PARTIAL_SIGNATURE_BYTES>(
                &pp,
            )?;
        Ok(Self(point))
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

#[test]
fn threshold_works() {
    use crate::MockRng;
    use rand_core::{RngCore, SeedableRng};

    let seed = [3u8; 16];
    let mut rng = MockRng::from_seed(seed);
    let sk = SecretKey::random(&mut rng).unwrap();
    let pk = PublicKey::from(&sk);

    let res_shares = sk.split::<MockRng, 2, 3>(&mut rng);
    assert!(res_shares.is_ok());
    let shares = res_shares.unwrap();
    let mut msg = [0u8; 12];
    rng.fill_bytes(&mut msg);

    let mut sigs = [PartialSignature::default(); 3];
    for (i, share) in shares.iter().enumerate() {
        let opt = PartialSignature::new(share, &msg);
        assert!(opt.is_some());
        sigs[i] = opt.unwrap();
    }

    // Try all combinations to make sure they verify
    for i in 0..3 {
        for j in 0..3 {
            if i == j {
                continue;
            }
            let res = Signature::from_partials::<2, 3>(&[sigs[i], sigs[j]]);
            assert!(res.is_ok());
            let sig = res.unwrap();
            assert_eq!(sig.verify(pk, msg).unwrap_u8(), 1);
        }
    }
}
