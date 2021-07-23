use super::SecretKey;
use bls12_381_plus::{G1Affine, G1Projective};
use core::{
    fmt::{self, Display},
    ops::{BitOr, Not},
};
use group::Curve;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use subtle::{Choice, CtOption};

/// A BLS public key
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PublicKeyVt(pub G1Projective);

impl Default for PublicKeyVt {
    fn default() -> Self {
        Self(G1Projective::identity())
    }
}

impl Display for PublicKeyVt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl From<&SecretKey> for PublicKeyVt {
    fn from(s: &SecretKey) -> Self {
        Self(G1Projective::generator() * s.0)
    }
}

impl From<PublicKeyVt> for [u8; PublicKeyVt::BYTES] {
    fn from(pk: PublicKeyVt) -> Self {
        pk.to_bytes()
    }
}

impl<'a> From<&'a PublicKeyVt> for [u8; PublicKeyVt::BYTES] {
    fn from(pk: &'a PublicKeyVt) -> [u8; PublicKeyVt::BYTES] {
        pk.to_bytes()
    }
}

impl Serialize for PublicKeyVt {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(s)
    }
}

impl<'de> Deserialize<'de> for PublicKeyVt {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let p = G1Projective::deserialize(d)?;
        Ok(Self(p))
    }
}

impl PublicKeyVt {
    /// Number of bytes needed to represent the public key
    pub const BYTES: usize = 48;

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
        G1Affine::from_compressed(bytes).map(|p| Self(G1Projective::from(&p)))
    }
}
