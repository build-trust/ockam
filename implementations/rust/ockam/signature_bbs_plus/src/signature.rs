use crate::MessageGenerators;
use blake2::Blake2b;
use bls12_381_plus::{
    multi_miller_loop, G1Affine, G1Projective, G2Affine, G2Prepared, G2Projective, Scalar,
};
use core::convert::TryFrom;
use core::ops::Neg;
use digest::Digest;
use ff::Field;
use group::{Curve, Group};
use hmac_drbg::HmacDRBG;
use serde::{
    de::{Error as DError, SeqAccess, Visitor},
    ser::SerializeTuple,
    Deserialize, Deserializer, Serialize, Serializer,
};
use signature_bls::{PublicKey, SecretKey};
use signature_core::{error::Error, lib::*};
use subtle::{Choice, ConditionallySelectable, CtOption};
use typenum::U64;

/// A BBS+ signature
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Signature {
    pub(crate) a: G1Projective,
    pub(crate) e: Scalar,
    pub(crate) s: Scalar,
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
            a: G1Projective::identity(),
            e: Scalar::zero(),
            s: Scalar::zero(),
        }
    }
}

impl ConditionallySelectable for Signature {
    fn conditional_select(a: &Self, b: &Self, choice: Choice) -> Self {
        Self {
            a: G1Projective::conditional_select(&a.a, &b.a, choice),
            e: Scalar::conditional_select(&a.e, &b.e, choice),
            s: Scalar::conditional_select(&a.s, &b.s, choice),
        }
    }
}

impl Signature {
    /// The number of bytes in a signature
    pub const BYTES: usize = 112;

    /// Generate a new signature where all messages are known to the signer
    pub fn new<M>(sk: &SecretKey, generators: &MessageGenerators, msgs: M) -> Result<Self, Error>
    where
        M: AsRef<[Message]>,
    {
        let msgs = msgs.as_ref();
        if generators.len() < msgs.len() {
            return Err(Error::new(1, "not enough message generators"));
        }
        if sk.0.is_zero() {
            return Err(Error::new(2, "invalid secret key"));
        }

        let mut hasher = Blake2b::new();
        hasher.update(generators.h0.to_affine().to_uncompressed());
        for i in 0..generators.len() {
            hasher.update(generators.get(i).to_affine().to_uncompressed());
        }
        for m in msgs {
            hasher.update(m.to_bytes())
        }
        let nonce = hasher.finalize();

        let mut drbg = HmacDRBG::<Blake2b>::new(&sk.to_bytes()[..], &nonce[..], &[]);
        // Should yield non-zero values for `e` and `s`, very small likelihood of it being zero
        let e = Scalar::from_bytes_wide(
            &<[u8; 64]>::try_from(&drbg.generate::<U64>(Some(&[1u8]))[..]).unwrap(),
        );
        let s = Scalar::from_bytes_wide(
            &<[u8; 64]>::try_from(&drbg.generate::<U64>(Some(&[2u8]))[..]).unwrap(),
        );
        let b = Self::compute_b(s, msgs, generators);
        let exp = (e + sk.0).invert().unwrap();

        Ok(Self { a: b * exp, e, s })
    }

    /// Verify a signature
    pub fn verify<M>(&self, pk: &PublicKey, generators: &MessageGenerators, msgs: M) -> Choice
    where
        M: AsRef<[Message]>,
    {
        let msgs = msgs.as_ref();
        if generators.len() < msgs.len() {
            return Choice::from(0);
        }
        // Identity point will always return true which is not what we want
        if pk.0.is_identity().unwrap_u8() == 1 {
            return Choice::from(0);
        }

        let a = G2Projective::generator() * self.e + pk.0;
        let b = Self::compute_b(self.s, msgs, generators).neg();

        multi_miller_loop(&[
            (&self.a.to_affine(), &G2Prepared::from(a.to_affine())),
            (&b.to_affine(), &G2Prepared::from(G2Affine::generator())),
        ])
        .final_exponentiation()
        .is_identity()
    }

    /// Get the byte representation of this signature
    pub fn to_bytes(&self) -> [u8; Self::BYTES] {
        let mut bytes = [0u8; Self::BYTES];
        bytes[0..48].copy_from_slice(&self.a.to_affine().to_compressed());
        let mut e = self.e.to_bytes();
        e.reverse();
        bytes[48..80].copy_from_slice(&e[..]);
        let mut s = self.s.to_bytes();
        s.reverse();
        bytes[80..112].copy_from_slice(&s[..]);
        bytes
    }

    /// Convert a byte sequence into a signature
    pub fn from_bytes(data: &[u8; Self::BYTES]) -> CtOption<Self> {
        let aa = G1Affine::from_compressed(&<[u8; 48]>::try_from(&data[0..48]).unwrap())
            .map(G1Projective::from);
        let mut e_bytes = <[u8; 32]>::try_from(&data[48..80]).unwrap();
        e_bytes.reverse();
        let ee = Scalar::from_bytes(&e_bytes);
        let mut s_bytes = <[u8; 32]>::try_from(&data[80..112]).unwrap();
        s_bytes.reverse();
        let ss = Scalar::from_bytes(&s_bytes);

        aa.and_then(|a| {
            ee.and_then(|e| ss.and_then(|s| CtOption::new(Signature { a, e, s }, Choice::from(1))))
        })
    }

    /// computes g1 + s * h0 + msgs[0] * h[0] + msgs[1] * h[1] ...
    /// hard limit at 128 for no-std
    pub(crate) fn compute_b(
        s: Scalar,
        msgs: &[Message],
        generators: &MessageGenerators,
    ) -> G1Projective {
        // Can't go more than 128, but that's quite a bit
        let points = [G1Projective::generator(), generators.h0]
            .iter()
            .copied()
            .chain(generators.iter())
            .collect::<Vec<G1Projective, 128>>();
        let mut scalars = [Scalar::one(), s]
            .iter()
            .copied()
            .chain(msgs.iter().map(|c| c.0))
            .collect::<Vec<Scalar, 128>>();

        G1Projective::sum_of_products_in_place(&points[..], &mut scalars[..])
    }
}
