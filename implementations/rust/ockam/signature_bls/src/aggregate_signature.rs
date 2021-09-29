use crate::{PublicKey, Signature};
use bls12_381_plus::{G1Affine, G1Projective, G2Affine};
use core::{
    fmt::{self, Display},
    ops::{BitOr, Neg, Not},
};
use group::{Curve, Group};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use subtle::{Choice, CtOption};

/// Represents a BLS signature in G1 for multiple signatures that signed the different messages
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AggregateSignature(pub(crate) G1Projective);

impl Display for AggregateSignature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Default for AggregateSignature {
    fn default() -> Self {
        Self(G1Projective::identity())
    }
}

impl From<&[Signature]> for AggregateSignature {
    fn from(sigs: &[Signature]) -> Self {
        let mut g = G1Projective::identity();
        for s in sigs {
            g += s.0;
        }
        Self(g)
    }
}

impl Serialize for AggregateSignature {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(s)
    }
}

impl<'de> Deserialize<'de> for AggregateSignature {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let p = G1Projective::deserialize(d)?;
        Ok(Self(p))
    }
}

impl AggregateSignature {
    /// Number of bytes needed to represent the signature
    pub const BYTES: usize = 48;

    /// Check if this signature is valid
    pub fn is_valid(&self) -> Choice {
        self.0.is_identity().not().bitor(self.0.is_on_curve())
    }

    /// Check if this signature is invalid
    pub fn is_invalid(&self) -> Choice {
        self.0.is_identity().bitor(self.0.is_on_curve().not())
    }

    /// Verify this multi signature is over `msg` with the multi public key
    pub fn verify<B: AsRef<[u8]>>(&self, data: &[(PublicKey, B)]) -> Choice {
        if self.is_invalid().unwrap_u8() == 1 {
            return Choice::from(0u8);
        }

        #[cfg(not(feature = "alloc"))]
        fn core_aggregate_verify<B: AsRef<[u8]>>(
            sig: &G1Projective,
            data: &[(PublicKey, B)],
        ) -> Choice {
            use bls12_381_plus::{pairing, Gt};

            let mut res = Gt::identity();
            for (key, msg) in data {
                if key.is_invalid().unwrap_u8() == 1 {
                    return Choice::from(0u8);
                }
                let a = Signature::hash_msg(msg.as_ref());
                res += pairing(&a.to_affine(), &key.0.to_affine());
            }
            res += pairing(&sig.to_affine(), &G2Affine::generator().neg());
            res.is_identity()
        }
        #[cfg(feature = "alloc")]
        fn core_aggregate_verify<B: AsRef<[u8]>>(
            sig: &G1Projective,
            data: &[(PublicKey, B)],
        ) -> Choice {
            use alloc::vec::Vec;
            use bls12_381_plus::{multi_miller_loop, G2Prepared};

            if data.iter().any(|(k, _)| k.is_invalid().unwrap_u8() == 1) {
                return Choice::from(0u8);
            }

            let mut data = data
                .iter()
                .map(|(key, m)| {
                    (
                        Signature::hash_msg(m.as_ref()).to_affine(),
                        G2Prepared::from(key.0.to_affine()),
                    )
                })
                .collect::<Vec<(G1Affine, G2Prepared)>>();

            data.push((
                sig.to_affine(),
                G2Prepared::from(G2Affine::generator().neg()),
            ));
            // appease borrow checker
            let t = data
                .iter()
                .map(|(p1, p2)| (p1, p2))
                .collect::<Vec<(&G1Affine, &G2Prepared)>>();
            multi_miller_loop(t.as_slice())
                .final_exponentiation()
                .is_identity()
        }
        core_aggregate_verify(&self.0, data)
    }

    /// Get the byte sequence that represents this aggregated signature
    pub fn to_bytes(self) -> [u8; Self::BYTES] {
        self.0.to_affine().to_compressed()
    }

    /// Convert a big-endian representation of the aggregated signature
    pub fn from_bytes(bytes: &[u8; Self::BYTES]) -> CtOption<Self> {
        let mut t = [0u8; Self::BYTES];
        t.copy_from_slice(bytes);
        G1Affine::from_compressed(&t).map(|p| Self(G1Projective::from(&p)))
    }
}
