use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

/// Timestamp in seconds (UTC)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Encode, Decode, Serialize, Deserialize)]
#[rustfmt::skip]
#[cbor(transparent)]
#[serde(transparent)]
pub struct TimestampInSeconds(#[n(0)] pub u64);

impl TimestampInSeconds {
    /// Add a Duration to a Timestamp
    pub fn add(&self, duration: DurationInSeconds) -> TimestampInSeconds {
        TimestampInSeconds(self.0 + duration.0)
    }
}

/// Duration in seconds
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Encode, Decode, Serialize, Deserialize)]
#[rustfmt::skip]
#[cbor(transparent)]
#[serde(transparent)]
pub struct DurationInSeconds(#[n(0)] pub u64);
