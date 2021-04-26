use crate::{MessageGenerators, Signature, MAX_MSGS};
use blake2::Blake2b;
use bls12_381_plus::{G1Projective, Scalar};
use core::convert::TryFrom;
use digest::Digest;
use ff::Field;
use group::Curve;
use hmac_drbg::HmacDRBG;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use signature_bls::SecretKey;
use signature_core::{error::Error, lib::*};
use subtle::CtOption;
use typenum::U64;

/// A BBS+ blind signature
/// structurally identical to `Signature` but is used to
/// help with misuse and confusion.
///
/// 1 or more messages have been hidden by the signature recipient
/// so the signer only knows a subset of the messages to be signed
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct BlindSignature {
    pub(crate) a: G1Projective,
    pub(crate) e: Scalar,
    pub(crate) s: Scalar,
}

impl Serialize for BlindSignature {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let sig = Signature {
            a: self.a,
            e: self.e,
            s: self.s,
        };
        sig.serialize(s)
    }
}

impl<'de> Deserialize<'de> for BlindSignature {
    fn deserialize<D>(d: D) -> Result<BlindSignature, D::Error>
    where
        D: Deserializer<'de>,
    {
        let sig = Signature::deserialize(d)?;
        Ok(Self {
            a: sig.a,
            e: sig.e,
            s: sig.s,
        })
    }
}

impl BlindSignature {
    /// The number of bytes in a signature
    pub const BYTES: usize = 112;

    /// Generate a blind signature where only a subset of messages are known to the signer
    /// The rest are encoded as a commitment
    pub fn new(
        commitment: Commitment,
        sk: &SecretKey,
        generators: &MessageGenerators,
        msgs: &[(usize, Message)],
    ) -> Result<Self, Error> {
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
        for (_, m) in msgs.iter() {
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

        // Can't go more than 128, but that's quite a bit
        let mut points = [G1Projective::identity(); MAX_MSGS];
        let mut scalars = [Scalar::one(); MAX_MSGS];

        points[0] = commitment.0;
        points[1] = G1Projective::generator();
        points[2] = generators.h0;
        scalars[2] = s;

        let mut i = 3;
        for (idx, m) in msgs.iter() {
            points[i] = generators.get(*idx);
            scalars[i] = m.0;
            i += 1;
        }

        let b = G1Projective::sum_of_products(&points[..i], &scalars[..i]);
        let exp = (e + sk.0).invert().unwrap();

        Ok(Self { a: b * exp, e, s })
    }

    /// Once signature on committed attributes (blind signature) is received, the signature needs to be unblinded.
    /// Takes the blinding factor used in the commitment.
    pub fn to_unblinded(self, blinding: SignatureBlinding) -> Signature {
        Signature {
            a: self.a,
            e: self.e,
            s: self.s + blinding.0,
        }
    }

    /// Get the byte representation of this signature
    pub fn to_bytes(&self) -> [u8; Self::BYTES] {
        let sig = Signature {
            a: self.a,
            e: self.e,
            s: self.s,
        };
        sig.to_bytes()
    }

    /// Convert a byte sequence into a signature
    pub fn from_bytes(data: &[u8; Self::BYTES]) -> CtOption<Self> {
        Signature::from_bytes(data).map(|sig| Self {
            a: sig.a,
            e: sig.e,
            s: sig.s,
        })
    }
}
