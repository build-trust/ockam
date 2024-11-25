use core::fmt::Display;
use minicbor::{CborLen, Decode, Encode};
use serde::{Deserialize, Serialize};

/// Timestamp in seconds (UTC)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Encode, Decode, CborLen, Serialize, Deserialize)]
#[rustfmt::skip]
#[cbor(transparent)]
#[serde(transparent)]
pub struct TimestampInSeconds(#[n(0)] pub u64);

impl Display for TimestampInSeconds {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // Convert to human-readable time
        let date = chrono::DateTime::from_timestamp(self.0 as i64, 0).ok_or(core::fmt::Error)?;
        write!(f, "{}", date)
    }
}
