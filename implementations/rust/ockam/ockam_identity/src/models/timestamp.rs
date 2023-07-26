use crate::IdentityError;
use minicbor::{Decode, Encode};
use ockam_core::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Encode, Decode, Serialize, Deserialize)]
#[rustfmt::skip]
#[cbor(transparent)]
#[serde(transparent)]
pub struct TimestampInSeconds(#[n(0)] u64);

impl TimestampInSeconds {
    // pub(crate) fn add_seconds(&self, seconds: u64) -> Self {
    //     Self(self.0.saturating_add(seconds))
    // }

    // /// Return the time elapsed between this timestamp and a previous one
    // pub fn elapsed(&self, since: Self) -> Option<Duration> {
    //     (self.0 >= since.0).then(|| Duration::from_secs(self.0 - since.0))
    // }

    /// Return the timestamp value as a number of seconds since the UNIX epoch time
    pub fn unix_time(&self) -> u64 {
        self.0
    }
}

// FIXME

/// Create a new timestamp using the system time
#[cfg(feature = "std")]
pub fn now() -> Result<TimestampInSeconds> {
    if let Ok(now) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(TimestampInSeconds(now.as_secs()))
    } else {
        return Err(IdentityError::InvalidInternalState.into());
    }
}

/// Create a new timestamp using the system time
#[cfg(not(feature = "std"))]
pub fn now() -> Result<TimestampInSeconds> {
    compile_error!("TimestampInSecond::now() implementation is required")
}
