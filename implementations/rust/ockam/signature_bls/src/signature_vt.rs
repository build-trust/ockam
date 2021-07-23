use crate::partial_signature_vt::PARTIAL_SIGNATURE_VT_BYTES;
use crate::{PartialSignatureVt, PublicKeyVt, SecretKey};
use bls12_381_plus::{
    multi_miller_loop, ExpandMsgXmd, G1Affine, G2Affine, G2Prepared, G2Projective, Scalar,
};
use core::{
    fmt::{self, Display},
    ops::Neg,
};
use ff::Field;
use group::{Curve, Group};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use subtle::{Choice, CtOption};
use vsss_rs::{Error, Shamir, Share};

/// Represents a BLS SignatureVt in G1 using the proof of possession scheme
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SignatureVt(pub(crate) G2Projective);

impl Display for SignatureVt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Default for SignatureVt {
    fn default() -> Self {
        Self(G2Projective::identity())
    }
}

impl Serialize for SignatureVt {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(s)
    }
}

impl<'de> Deserialize<'de> for SignatureVt {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let p = G2Projective::deserialize(d)?;
        Ok(Self(p))
    }
}

impl SignatureVt {
    /// Number of bytes needed to represent the SignatureVt
    pub const BYTES: usize = 96;
    /// The domain separation tag
    const DST: &'static [u8] = b"BLS_SIG_BLS12381G2_XMD:SHA-256_SSWU_RO_POP_";

    /// Create a new bls
    pub fn new<B: AsRef<[u8]>>(sk: &SecretKey, msg: B) -> Option<Self> {
        if sk.0.is_zero() {
            return None;
        }
        let a = Self::hash_msg(msg.as_ref());
        Some(Self(a * sk.0))
    }

    pub(crate) fn hash_msg(msg: &[u8]) -> G2Projective {
        G2Projective::hash::<ExpandMsgXmd<sha2::Sha256>>(msg, Self::DST)
    }

    /// Verify if the bls is over `msg` with `pk`
    pub fn verify<B: AsRef<[u8]>>(&self, pk: PublicKeyVt, msg: B) -> Choice {
        if pk.0.is_identity().unwrap_u8() == 1 || self.0.is_identity().unwrap_u8() == 1 {
            return Choice::from(0);
        }
        let a = Self::hash_msg(msg.as_ref());
        let g1 = G1Affine::generator().neg();

        multi_miller_loop(&[
            (&pk.0.to_affine(), &G2Prepared::from(a.to_affine())),
            (&g1, &G2Prepared::from(self.0.to_affine())),
        ])
        .final_exponentiation()
        .is_identity()
    }

    /// Get the byte sequence that represents this SignatureVt
    pub fn to_bytes(self) -> [u8; Self::BYTES] {
        self.0.to_affine().to_compressed()
    }

    /// Convert a big-endian representation of the SignatureVt
    pub fn from_bytes(bytes: &[u8; Self::BYTES]) -> CtOption<Self> {
        G2Affine::from_compressed(bytes).map(|p| Self(G2Projective::from(&p)))
    }

    /// Combine partial signatures into a completed signature
    pub fn from_partials<const T: usize, const N: usize>(
        partials: &[PartialSignatureVt],
    ) -> Result<Self, Error> {
        if T > partials.len() {
            return Err(Error::SharingLimitLessThanThreshold);
        }
        let mut pp = [Share::<PARTIAL_SIGNATURE_VT_BYTES>::default(); T];
        for i in 0..T {
            pp[i] = partials[i].0;
        }
        let point = Shamir::<T, N>::combine_shares_group::<
            Scalar,
            G2Projective,
            PARTIAL_SIGNATURE_VT_BYTES,
        >(&pp)?;
        Ok(Self(point))
    }
}

#[test]
fn signature_vt_works() {
    use crate::MockRng;
    use rand_core::{RngCore, SeedableRng};

    let seed = [2u8; 16];
    let mut rng = MockRng::from_seed(seed);
    let sk = SecretKey::random(&mut rng).unwrap();
    let mut msg = [0u8; 12];
    rng.fill_bytes(&mut msg);
    let sig = SignatureVt::new(&sk, msg).unwrap();
    let pk = PublicKeyVt::from(&sk);
    assert_eq!(sig.verify(pk, msg).unwrap_u8(), 1);
}

#[test]
fn threshold_works() {
    use crate::MockRng;
    use rand_core::{RngCore, SeedableRng};

    let seed = [3u8; 16];
    let mut rng = MockRng::from_seed(seed);
    let sk = SecretKey::random(&mut rng).unwrap();
    let pk = PublicKeyVt::from(&sk);

    let res_shares = sk.split::<MockRng, 2, 3>(&mut rng);
    assert!(res_shares.is_ok());
    let shares = res_shares.unwrap();
    let mut msg = [0u8; 12];
    rng.fill_bytes(&mut msg);

    let mut sigs = [PartialSignatureVt::default(); 3];
    for (i, share) in shares.iter().enumerate() {
        let opt = PartialSignatureVt::new(share, &msg);
        assert!(opt.is_some());
        sigs[i] = opt.unwrap();
    }

    // Try all combinations to make sure they verify
    for i in 0..3 {
        for j in 0..3 {
            if i == j {
                continue;
            }
            let res = SignatureVt::from_partials::<2, 3>(&[sigs[i], sigs[j]]);
            assert!(res.is_ok());
            let sig = res.unwrap();
            assert_eq!(sig.verify(pk, msg).unwrap_u8(), 1);
        }
    }
}
