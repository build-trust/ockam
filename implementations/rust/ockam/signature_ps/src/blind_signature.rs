use crate::{SecretKey, Signature};
use blake2::Blake2b;
use bls12_381_plus::{G1Affine, G1Projective, Scalar};
use core::convert::TryFrom;
use digest::Digest;
use group::Curve;
use hmac_drbg::HmacDRBG;
use serde::{
    de::{Error as DError, SeqAccess, Visitor},
    ser::SerializeTuple,
    Deserialize, Deserializer, Serialize, Serializer,
};
use signature_core::{constants::*, error::Error, lib::*, util::*};
use subtle::{Choice, CtOption};
use typenum::U64;

/// A PS blind signature
/// structurally identical to `Signature` but is used to
/// help with misuse and confusion.
///
/// 1 or more messages have been hidden by the signature recipient
/// so the signer only knows a subset of the messages to be signed
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct BlindSignature {
    pub(crate) sigma_1: G1Projective,
    pub(crate) sigma_2: G1Projective,
    pub(crate) m_tick: Scalar,
}

impl Serialize for BlindSignature {
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

impl<'de> Deserialize<'de> for BlindSignature {
    fn deserialize<D>(d: D) -> Result<BlindSignature, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ArrayVisitor;

        impl<'de> Visitor<'de> for ArrayVisitor {
            type Value = BlindSignature;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "expected byte array")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<BlindSignature, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut arr = [0u8; BlindSignature::BYTES];

                #[allow(clippy::needless_range_loop)]
                for i in 0..arr.len() {
                    arr[i] = seq
                        .next_element()?
                        .ok_or_else(|| DError::invalid_length(i, &self))?;
                }
                let res = BlindSignature::from_bytes(&arr);
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

        d.deserialize_tuple(BlindSignature::BYTES, ArrayVisitor)
    }
}

impl Default for BlindSignature {
    fn default() -> Self {
        Self {
            sigma_1: G1Projective::identity(),
            sigma_2: G1Projective::identity(),
            m_tick: Scalar::zero(),
        }
    }
}

impl BlindSignature {
    /// The number of bytes in a signature
    pub const BYTES: usize = 128;

    /// Generate a new signature where all messages are known to the signer
    pub fn new(
        commitment: Commitment,
        sk: &SecretKey,
        msgs: &[(usize, Message)],
    ) -> Result<Self, Error> {
        if sk.y.len() < msgs.len() {
            return Err(Error::new(1, "secret key is not big enough"));
        }
        if sk.is_invalid() {
            return Err(Error::new(1, "invalid secret key"));
        }

        let mut hasher = Blake2b::new();
        hasher.update(
            (G1Projective::generator() * sk.w)
                .to_affine()
                .to_uncompressed(),
        );
        hasher.update(
            (G1Projective::generator() * sk.x)
                .to_affine()
                .to_uncompressed(),
        );
        for y in &sk.y {
            hasher.update(
                (G1Projective::generator() * y)
                    .to_affine()
                    .to_uncompressed(),
            );
        }
        for (_, m) in msgs.iter() {
            hasher.update(m.to_bytes())
        }
        let nonce = hasher.finalize_reset();

        hasher.update(sk.w.to_bytes());
        hasher.update(sk.x.to_bytes());
        for y in &sk.y {
            hasher.update(y.to_bytes());
        }
        let entropy = hasher.finalize();

        let mut drbg = HmacDRBG::<Blake2b>::new(&entropy[..], &nonce[..], &[]);
        // Should yield non-zero values for `u` and m', very small likelihood of it being zero
        let u = Scalar::from_bytes_wide(
            &<[u8; 64]>::try_from(&drbg.generate::<U64>(Some(&[1u8]))[..]).unwrap(),
        );
        let m_tick = Scalar::from_bytes_wide(
            &<[u8; 64]>::try_from(&drbg.generate::<U64>(Some(&[2u8]))[..]).unwrap(),
        );
        let sigma_1 = G1Projective::generator() * u;

        let mut exp = sk.x + m_tick * sk.w;
        for (i, msg) in msgs {
            exp += sk.y[*i] * msg.0;
        }
        let mut sigma_2 = (G1Projective::generator() * exp) + commitment.0;
        sigma_2 *= u;
        Ok(Self {
            sigma_1,
            sigma_2,
            m_tick,
        })
    }

    /// Once signature on committed attributes (blind signature) is received, the signature needs to be unblinded.
    /// Takes the blinding factor used in the commitment.
    pub fn to_unblinded(self, blinding: SignatureBlinding) -> Signature {
        Signature {
            sigma_1: self.sigma_1,
            sigma_2: self.sigma_2 - (self.sigma_1 * blinding.0),
            m_tick: self.m_tick,
        }
    }

    /// Get the byte representation of this signature
    pub fn to_bytes(&self) -> [u8; Self::BYTES] {
        let mut bytes = [0u8; Self::BYTES];
        bytes[..48].copy_from_slice(&self.sigma_1.to_affine().to_compressed());
        bytes[48..96].copy_from_slice(&self.sigma_2.to_affine().to_compressed());

        #[allow(clippy::out_of_bounds_indexing)] // TODO - BUG
        bytes[96..128].copy_from_slice(&scalar_to_bytes(self.m_tick));
        bytes
    }

    /// Convert a byte sequence into a signature
    pub fn from_bytes(data: &[u8; Self::BYTES]) -> CtOption<Self> {
        let s1 = G1Affine::from_compressed(slicer!(data, 0, 48, COMMITMENT_BYTES))
            .map(G1Projective::from);
        let s2 = G1Affine::from_compressed(slicer!(data, 48, 96, COMMITMENT_BYTES))
            .map(G1Projective::from);

        #[allow(clippy::out_of_bounds_indexing)] // TODO - BUG
        let m_t = scalar_from_bytes(slicer!(data, 96, 128, FIELD_BYTES));

        s1.and_then(|sigma_1| {
            s2.and_then(|sigma_2| {
                m_t.and_then(|m_tick| {
                    CtOption::new(
                        BlindSignature {
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
