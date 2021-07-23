use crate::{MultiPublicKey, PublicKey, Signature};
use bls12_381_plus::{G1Affine, G1Projective};
use core::{
    fmt::{self, Display},
    ops::{BitOr, Not},
};
use group::Curve;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use subtle::{Choice, CtOption};

/// Represents a BLS signature in G1 for multiple signatures that signed the same message
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MultiSignature(pub(crate) G1Projective);

impl Default for MultiSignature {
    fn default() -> Self {
        Self(G1Projective::identity())
    }
}

impl Display for MultiSignature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl From<&[Signature]> for MultiSignature {
    fn from(sigs: &[Signature]) -> Self {
        let mut g = G1Projective::identity();
        for s in sigs {
            g += s.0;
        }
        Self(g)
    }
}

impl Serialize for MultiSignature {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(s)
    }
}

impl<'de> Deserialize<'de> for MultiSignature {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let p = G1Projective::deserialize(d)?;
        Ok(Self(p))
    }
}

impl MultiSignature {
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
    pub fn verify<B: AsRef<[u8]>>(&self, public_key: MultiPublicKey, msg: B) -> Choice {
        Signature(self.0).verify(PublicKey(public_key.0), msg)
    }

    /// Get the byte sequence that represents this multisignature
    pub fn to_bytes(self) -> [u8; Self::BYTES] {
        self.0.to_affine().to_compressed()
    }

    /// Convert a big-endian representation of the multisignature
    pub fn from_bytes(bytes: &[u8; Self::BYTES]) -> CtOption<Self> {
        let mut t = [0u8; Self::BYTES];
        t.copy_from_slice(bytes);
        G1Affine::from_compressed(&t).map(|p| Self(G1Projective::from(&p)))
    }
}
