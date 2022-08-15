use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Unsigned integer implementation that supports serde_bare
/// serialization as a 64 bit `uint` type.
///
/// This is to avoid cross-platform and cross-language compatibility
/// inconsistencies that may be encountered by using Rust fixed-size
/// integers.
#[derive(Debug, PartialEq, Eq)]
pub struct Uint(serde_bare::Uint);

impl Uint {
    /// Return the underlying integer.
    pub fn u64(&self) -> u64 {
        self.0 .0
    }
}

impl Serialize for Uint {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Uint {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let u = serde_bare::Uint::deserialize(deserializer)?;
        Ok(Self(u))
    }
}

impl From<u64> for Uint {
    fn from(u: u64) -> Self {
        Self(serde_bare::Uint(u))
    }
}
