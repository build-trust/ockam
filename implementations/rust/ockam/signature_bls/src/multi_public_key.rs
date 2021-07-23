use crate::PublicKey;
use bls12_381_plus::{G2Affine, G2Projective};
use core::{
    fmt::{self, Display},
    ops::{BitOr, Not},
};
use group::Curve;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use subtle::{Choice, CtOption};

/// Represents multiple public keys into one that can be used to verify multisignatures
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MultiPublicKey(pub(crate) G2Projective);

impl From<&[PublicKey]> for MultiPublicKey {
    fn from(keys: &[PublicKey]) -> Self {
        let mut g = G2Projective::identity();
        for k in keys {
            g += k.0;
        }
        Self(g)
    }
}

impl Display for MultiPublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Default for MultiPublicKey {
    fn default() -> Self {
        Self(G2Projective::identity())
    }
}

impl Serialize for MultiPublicKey {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(s)
    }
}

impl<'de> Deserialize<'de> for MultiPublicKey {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let p = G2Projective::deserialize(d)?;
        Ok(Self(p))
    }
}

impl MultiPublicKey {
    /// Number of bytes needed to represent the multi public key
    pub const BYTES: usize = 96;

    /// Check if this signature is valid
    pub fn is_valid(&self) -> Choice {
        self.0.is_identity().not().bitor(self.0.is_on_curve())
    }

    /// Check if this signature is invalid
    pub fn is_invalid(&self) -> Choice {
        self.0.is_identity().bitor(self.0.is_on_curve().not())
    }

    /// Get the byte representation of this key
    pub fn to_bytes(self) -> [u8; Self::BYTES] {
        self.0.to_affine().to_compressed()
    }

    /// Convert a big-endian representation of the multi public key
    pub fn from_bytes(bytes: &[u8; Self::BYTES]) -> CtOption<Self> {
        let mut t = [0u8; Self::BYTES];
        t.copy_from_slice(bytes);
        G2Affine::from_compressed(&t).map(|p| Self(G2Projective::from(&p)))
    }
}
