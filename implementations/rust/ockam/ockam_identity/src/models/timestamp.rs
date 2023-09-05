use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

/// Timestamp in seconds (UTC)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Encode, Decode, Serialize, Deserialize)]
#[rustfmt::skip]
#[cbor(transparent)]
#[serde(transparent)]
pub struct TimestampInSeconds(#[n(0)] pub(crate) u64);

impl TimestampInSeconds {
    /// Create a new [`TimestampInSeconds`]
    pub fn new(timestamp: u64) -> Self {
        Self(timestamp)
    }
}

impl core::ops::Deref for TimestampInSeconds {
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<u64> for TimestampInSeconds {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl core::ops::Add<TimestampInSeconds> for TimestampInSeconds {
    type Output = TimestampInSeconds;

    fn add(self, rhs: TimestampInSeconds) -> Self::Output {
        TimestampInSeconds(self.0 + rhs.0)
    }
}
