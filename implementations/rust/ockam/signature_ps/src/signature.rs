use crate::{PublicKey, SecretKey};
use blake2::{Blake2b, VarBlake2b};
use bls12_381_plus::{
    multi_miller_loop, ExpandMsgXmd, G1Affine, G1Projective, G2Affine, G2Prepared, G2Projective,
    Scalar,
};
use core::convert::TryFrom;
use digest::{Update, VariableOutput};
use group::{Curve, Group};
use serde::{
    de::{Error as DError, SeqAccess, Visitor},
    ser::SerializeTuple,
    Deserialize, Deserializer, Serialize, Serializer,
};
use signature_core::{constants::*, error::Error, lib::*, util::*};
use subtle::{Choice, CtOption};

/// A Pointcheval Saunders signature
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Signature {
    pub(crate) sigma_1: G1Projective,
    pub(crate) sigma_2: G1Projective,
    pub(crate) m_tick: Scalar,
}

impl Serialize for Signature {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let bytes = self.to_bytes();
        let mut seq = s.serialize_tuple(bytes.len())?;
        for b in &bytes {
            seq.serialize_element(b)?;
        }
        seq.end()
    }
}

impl<'de> Deserialize<'de> for Signature {
    fn deserialize<D>(d: D) -> Result<Signature, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ArrayVisitor;

        impl<'de> Visitor<'de> for ArrayVisitor {
            type Value = Signature;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "expected byte array")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Signature, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut arr = [0u8; Signature::BYTES];
                #[allow(clippy::needless_range_loop)]
                for i in 0..arr.len() {
                    arr[i] = seq
                        .next_element()?
                        .ok_or_else(|| DError::invalid_length(i, &self))?;
                }
                let res = Signature::from_bytes(&arr);
                if res.is_some().unwrap_u8() == 1 {
                    Ok(res.unwrap())
                } else {
                    Err(DError::invalid_value(
                        serde::de::Unexpected::Bytes(&arr),
                        &self,
                    ))
                }
            }
        }

        d.deserialize_tuple(Signature::BYTES, ArrayVisitor)
    }
}

impl Default for Signature {
    fn default() -> Self {
        Self {
            sigma_1: G1Projective::identity(),
            sigma_2: G1Projective::identity(),
            m_tick: Scalar::zero(),
        }
    }
}

impl Signature {
    /// The number of bytes in a signature
    pub const BYTES: usize = 128;
    const DST: &'static [u8] = b"PS_SIG_BLS12381G1_XMD:BLAKE2B_SSWU_RO_";

    /// Generate a new signature where all messages are known to the signer
    pub fn new<M>(sk: &SecretKey, msgs: M) -> Result<Self, Error>
    where
        M: AsRef<[Message]>,
    {
        let msgs = msgs.as_ref();
        if sk.y.len() < msgs.len() {
            return Err(Error::new(1, "secret key is not big enough"));
        }
        if sk.is_invalid() {
            return Err(Error::new(1, "invalid secret key"));
        }

        let mut hasher = VarBlake2b::new(48).unwrap();
        for m in msgs {
            hasher.update(m.to_bytes());
        }
        let mut out = [0u8; 48];
        hasher.finalize_variable(|r| {
            out.copy_from_slice(r);
        });
        let m_tick = Scalar::from_okm(&out);
        let sigma_1 =
            G1Projective::hash::<ExpandMsgXmd<Blake2b>>(&m_tick.to_bytes()[..], Self::DST);
        let mut exp = sk.x + sk.w * m_tick;

        #[allow(clippy::needless_range_loop)]
        for i in 0..msgs.len() {
            exp += sk.y[i] * msgs[i].0;
        }
        let sigma_2 = sigma_1 * exp;
        Ok(Self {
            sigma_1,
            sigma_2,
            m_tick,
        })
    }

    /// Verify a signature
    pub fn verify<M>(&self, pk: &PublicKey, msgs: M) -> Choice
    where
        M: AsRef<[Message]>,
    {
        let msgs = msgs.as_ref();
        if pk.y.len() < msgs.len() {
            return Choice::from(0);
        }
        if pk.is_invalid().unwrap_u8() == 1 {
            return Choice::from(0);
        }

        let mut points = Vec::<G2Projective, 130>::new();
        let mut scalars = Vec::<Scalar, 130>::new();
        points.push(pk.x).expect(ALLOC_MSG);
        scalars.push(Scalar::one()).expect(ALLOC_MSG);

        points.push(pk.w).expect(ALLOC_MSG);
        scalars.push(self.m_tick).expect(ALLOC_MSG);

        #[allow(clippy::needless_range_loop)]
        for i in 0..msgs.len() {
            points.push(pk.y[i]).expect(ALLOC_MSG);
            scalars.push(msgs[i].0).expect(ALLOC_MSG);
        }

        // Y_m = X_tilde * W_tilde^m' * Y_tilde[1]^m_1 * Y_tilde[2]^m_2 * ...Y_tilde[i]^m_i
        let y_m = G2Projective::sum_of_products_in_place(points.as_ref(), scalars.as_mut());

        // e(sigma_1, Y_m) == e(sigma_2, G2) or
        // e(sigma_1 + sigma_2, Y_m - G2) == GT_1
        multi_miller_loop(&[
            (
                &self.sigma_1.to_affine(),
                &G2Prepared::from(y_m.to_affine()),
            ),
            (
                &self.sigma_2.to_affine(),
                &G2Prepared::from(-G2Affine::generator()),
            ),
        ])
        .final_exponentiation()
        .is_identity()
    }

    /// Get the byte representation of this signature
    pub fn to_bytes(&self) -> [u8; Self::BYTES] {
        let mut bytes = [0u8; Self::BYTES];
        bytes[..48].copy_from_slice(&self.sigma_1.to_affine().to_compressed());
        bytes[48..96].copy_from_slice(&self.sigma_2.to_affine().to_compressed());
        bytes[96..128].copy_from_slice(&scalar_to_bytes(self.m_tick));
        bytes
    }

    /// Convert a byte sequence into a signature
    pub fn from_bytes(data: &[u8; Self::BYTES]) -> CtOption<Self> {
        let s1 = G1Affine::from_compressed(slicer!(data, 0, 48, COMMITMENT_BYTES))
            .map(G1Projective::from);
        let s2 = G1Affine::from_compressed(slicer!(data, 48, 96, COMMITMENT_BYTES))
            .map(G1Projective::from);
        let m_t = scalar_from_bytes(slicer!(data, 96, 128, FIELD_BYTES));

        s1.and_then(|sigma_1| {
            s2.and_then(|sigma_2| {
                m_t.and_then(|m_tick| {
                    CtOption::new(
                        Signature {
                            sigma_1,
                            sigma_2,
                            m_tick,
                        },
                        Choice::from(1),
                    )
                })
            })
        })
    }
}
