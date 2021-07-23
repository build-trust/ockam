use super::SecretKey;
use bls12_381_plus::{G2Affine, G2Projective};
use core::{
    fmt::{self, Display},
    ops::{BitOr, Not},
};
use group::Curve;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use subtle::{Choice, CtOption};

/// A BLS public key
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PublicKey(pub G2Projective);

impl Default for PublicKey {
    fn default() -> Self {
        Self(G2Projective::identity())
    }
}

impl Display for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl From<&SecretKey> for PublicKey {
    fn from(s: &SecretKey) -> Self {
        Self(G2Projective::generator() * s.0)
    }
}

impl From<PublicKey> for [u8; PublicKey::BYTES] {
    fn from(pk: PublicKey) -> Self {
        pk.to_bytes()
    }
}

impl<'a> From<&'a PublicKey> for [u8; PublicKey::BYTES] {
    fn from(pk: &'a PublicKey) -> [u8; PublicKey::BYTES] {
        pk.to_bytes()
    }
}

impl Serialize for PublicKey {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(s)
    }
}

impl<'de> Deserialize<'de> for PublicKey {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let p = G2Projective::deserialize(d)?;
        Ok(Self(p))
    }
}

impl PublicKey {
    /// Number of bytes needed to represent the public key
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

    /// Convert a big-endian representation of the public key
    pub fn from_bytes(bytes: &[u8; Self::BYTES]) -> CtOption<Self> {
        G2Affine::from_compressed(bytes).map(|p| Self(G2Projective::from(&p)))
    }
}
