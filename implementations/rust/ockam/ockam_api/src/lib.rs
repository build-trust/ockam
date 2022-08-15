pub mod auth;
pub mod authenticator;
pub mod cloud;
pub mod config;
pub mod echoer;
pub mod error;
pub mod identity;
pub mod nodes;
pub mod old;
pub mod signer;
pub mod uppercase;
pub mod vault;

mod util;
pub use util::*;

#[cfg(feature = "lmdb")]
pub mod lmdb;

#[macro_use]
extern crate tracing;

pub const SCHEMA: &str = core::include_str!("../schema.cddl");

use minicbor::{Decode, Encode};

/// A Unix timestamp (seconds since 1970-01-01 00:00:00Z)
#[cfg(feature = "std")]
#[derive(Debug, Clone, Copy, Encode, Decode, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cbor(transparent)]
pub struct Timestamp(#[n(0)] u64);

#[cfg(feature = "std")]
impl Timestamp {
    pub fn now() -> Option<Self> {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .map(|d| Timestamp(d.as_secs()))
    }

    pub fn elapsed(&self, since: Timestamp) -> Option<core::time::Duration> {
        (self.0 >= since.0).then(|| core::time::Duration::from_secs(self.0 - since.0))
    }
}

#[cfg(feature = "std")]
impl From<Timestamp> for u64 {
    fn from(t: Timestamp) -> Self {
        t.0
    }
}
