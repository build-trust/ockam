use crate::{MultiPublicKeyVt, PublicKeyVt, SignatureVt};
use bls12_381_plus::{G2Affine, G2Projective};
use core::{
    fmt::{self, Display},
    ops::{BitOr, Not},
};
use group::Curve;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use subtle::{Choice, CtOption};

/// Represents a BLS SignatureVt in G1 for multiple SignatureVts that signed the same message
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MultiSignatureVt(pub(crate) G2Projective);

impl Default for MultiSignatureVt {
    fn default() -> Self {
        Self(G2Projective::identity())
    }
}

impl Display for MultiSignatureVt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl From<&[SignatureVt]> for MultiSignatureVt {
    fn from(sigs: &[SignatureVt]) -> Self {
        let mut g = G2Projective::identity();
        for s in sigs {
            g += s.0;
        }
        Self(g)
    }
}

impl Serialize for MultiSignatureVt {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(s)
    }
}

impl<'de> Deserialize<'de> for MultiSignatureVt {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let p = G2Projective::deserialize(d)?;
        Ok(Self(p))
    }
}

impl MultiSignatureVt {
    /// Number of bytes needed to represent the SignatureVt
    pub const BYTES: usize = 96;

    /// Check if this signature is valid
    pub fn is_valid(&self) -> Choice {
        self.0.is_identity().not().bitor(self.0.is_on_curve())
    }

    /// Check if this signature is invalid
    pub fn is_invalid(&self) -> Choice {
        self.0.is_identity().bitor(self.0.is_on_curve().not())
    }

    /// Verify this multi SignatureVt is over `msg` with the multi public key
    pub fn verify<B: AsRef<[u8]>>(&self, public_key: MultiPublicKeyVt, msg: B) -> Choice {
        SignatureVt(self.0).verify(PublicKeyVt(public_key.0), msg)
    }

    /// Get the byte sequence that represents this multiSignatureVt
    pub fn to_bytes(self) -> [u8; Self::BYTES] {
        self.0.to_affine().to_compressed()
    }

    /// Convert a big-endian representation of the multiSignatureVt
    pub fn from_bytes(bytes: &[u8; Self::BYTES]) -> CtOption<Self> {
        let mut t = [0u8; Self::BYTES];
        t.copy_from_slice(bytes);
        G2Affine::from_compressed(&t).map(|p| Self(G2Projective::from(&p)))
    }
}
