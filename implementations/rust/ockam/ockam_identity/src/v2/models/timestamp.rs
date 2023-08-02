use core::ops::Deref;
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Encode, Decode, Serialize, Deserialize)]
#[rustfmt::skip]
#[cbor(transparent)]
#[serde(transparent)]
pub struct TimestampInSeconds(#[n(0)] u64);

impl TimestampInSeconds {
    pub fn new(timestamp: u64) -> Self {
        Self(timestamp)
    }
}

impl Deref for TimestampInSeconds {
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
