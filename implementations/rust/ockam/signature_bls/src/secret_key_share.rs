use core::fmt::{self, Display};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use vsss_rs::Share;
use zeroize::Zeroize;

/// A secret key share is field element 0 < `x` < `r`
/// where `r` is the curve order. See Section 4.3 in
/// <https://eprint.iacr.org/2016/663.pdf>
/// Must be combined with other secret key shares
/// to produce the completed key, or used for
/// creating partial signatures which can be
/// combined into a complete signature
#[derive(Clone, Debug, Default, Zeroize)]
pub struct SecretKeyShare(pub Share<SECRET_KEY_SHARE_BYTES>);

impl Drop for SecretKeyShare {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

impl From<Share<SECRET_KEY_SHARE_BYTES>> for SecretKeyShare {
    fn from(share: Share<SECRET_KEY_SHARE_BYTES>) -> Self {
        Self(share)
    }
}

impl<'a> From<&'a Share<SECRET_KEY_SHARE_BYTES>> for SecretKeyShare {
    fn from(share: &'a Share<SECRET_KEY_SHARE_BYTES>) -> Self {
        Self(*share)
    }
}

impl Display for SecretKeyShare {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for b in &self.0 .0 {
            b.fmt(f)?;
        }
        Ok(())
    }
}

impl From<SecretKeyShare> for [u8; SecretKeyShare::BYTES] {
    fn from(sk: SecretKeyShare) -> [u8; SecretKeyShare::BYTES] {
        sk.to_bytes()
    }
}

impl<'a> From<&'a SecretKeyShare> for [u8; SecretKeyShare::BYTES] {
    fn from(sk: &'a SecretKeyShare) -> [u8; SecretKeyShare::BYTES] {
        sk.to_bytes()
    }
}

impl Serialize for SecretKeyShare {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(s)
    }
}

impl<'de> Deserialize<'de> for SecretKeyShare {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let share = Share::<SECRET_KEY_SHARE_BYTES>::deserialize(d)?;
        Ok(Self(share))
    }
}

impl SecretKeyShare {
    /// Number of bytes needed to represent the secret key
    pub const BYTES: usize = SECRET_KEY_SHARE_BYTES;

    /// Is this share zero
    pub fn is_zero(&self) -> bool {
        let mut r = 0u8;
        for b in self.0.value() {
            r |= *b;
        }
        r == 0
    }

    /// Get the byte representation of this key
    pub fn to_bytes(&self) -> [u8; Self::BYTES] {
        self.0 .0
    }

    /// Convert a big-endian representation of the secret key.
    pub fn from_bytes(bytes: &[u8; Self::BYTES]) -> Self {
        Self(Share(*bytes))
    }
}

pub(crate) const SECRET_KEY_SHARE_BYTES: usize = 33;
