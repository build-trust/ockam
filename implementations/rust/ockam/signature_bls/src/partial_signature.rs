use crate::{SecretKeyShare, Signature};
use bls12_381_plus::{G1Affine, G1Projective, Scalar};
use core::{
    fmt::{self, Display},
    ops::{BitOr, Not},
};
use group::Curve;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use subtle::Choice;
use vsss_rs::Share;

/// Represents a BLS partial signature in G1 using the proof of possession scheme
#[derive(Clone, Copy, Debug, Default)]
pub struct PartialSignature(pub(crate) Share<PARTIAL_SIGNATURE_BYTES>);

impl Display for PartialSignature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for b in &self.0 .0 {
            b.fmt(f)?;
        }
        Ok(())
    }
}

impl From<Share<PARTIAL_SIGNATURE_BYTES>> for PartialSignature {
    fn from(share: Share<PARTIAL_SIGNATURE_BYTES>) -> Self {
        Self(share)
    }
}

impl<'a> From<&'a Share<PARTIAL_SIGNATURE_BYTES>> for PartialSignature {
    fn from(share: &'a Share<PARTIAL_SIGNATURE_BYTES>) -> Self {
        Self(*share)
    }
}

impl Serialize for PartialSignature {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(s)
    }
}

impl<'de> Deserialize<'de> for PartialSignature {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let p = Share::<PARTIAL_SIGNATURE_BYTES>::deserialize(d)?;
        Ok(Self(p))
    }
}

impl PartialSignature {
    /// Number of bytes needed to represent the signature
    pub const BYTES: usize = PARTIAL_SIGNATURE_BYTES;

    /// Create a new bls
    pub fn new<B: AsRef<[u8]>>(sk: &SecretKeyShare, msg: B) -> Option<Self> {
        if sk.is_zero() {
            return None;
        }
        let a = Signature::hash_msg(msg.as_ref());
        let t = <[u8; 32]>::try_from(sk.0.value()).unwrap();
        let res = Scalar::from_bytes(&t).map(|s| {
            let point = a * s;
            let mut bytes = [0u8; PARTIAL_SIGNATURE_BYTES];
            bytes[1..].copy_from_slice(&point.to_affine().to_compressed());
            bytes[0] = sk.0.identifier();
            Some(PartialSignature(Share(bytes)))
        });
        if res.is_some().unwrap_u8() == 1 {
            res.unwrap()
        } else {
            None
        }
    }

    /// Check if this partial signature is valid
    pub fn is_valid(&self) -> Choice {
        let t: [u8; 48] = <[u8; 48]>::try_from(self.0.value()).unwrap();
        let p = G1Affine::from_compressed(&t).map(G1Projective::from);
        p.map(|v| v.is_identity().not().bitor(v.is_on_curve()))
            .unwrap_or_else(|| Choice::from(0u8))
    }

    /// Check if this partial signature is invalid
    pub fn is_invalid(&self) -> Choice {
        let t: [u8; 48] = <[u8; 48]>::try_from(self.0.value()).unwrap();
        let p = G1Affine::from_compressed(&t).map(G1Projective::from);
        p.map(|v| v.is_identity().bitor(v.is_on_curve().not()))
            .unwrap_or_else(|| Choice::from(0u8))
    }

    /// Get the byte sequence that represents this partial signature
    pub fn to_bytes(self) -> [u8; Self::BYTES] {
        self.0 .0
    }

    /// Convert a big-endian representation of the partial signature
    pub fn from_bytes(bytes: &[u8; Self::BYTES]) -> Self {
        Self(Share(*bytes))
    }
}

pub(crate) const PARTIAL_SIGNATURE_BYTES: usize = 49;
