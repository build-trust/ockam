use ockam_core::compat::string::{String, ToString};

#[cfg(feature = "lease_proto_json")]
pub mod json_proto;

#[cfg(feature = "lease_proto_json")]
pub use json_proto::*;

use serde::{Deserialize, Serialize};
pub type TTL = usize;

#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Debug)]
pub struct Lease {
    value: String,
    ttl: TTL,
}

impl Lease {
    pub fn new<S: ToString>(value: S, ttl: usize) -> Self {
        Lease {
            value: value.to_string(),
            ttl,
        }
    }

    pub fn value(&self) -> &str {
        self.value.as_str()
    }

    pub fn ttl(&self) -> TTL {
        self.ttl
    }

    pub fn invalid(&self) -> bool {
        self.value.is_empty() || self.ttl == 0
    }

    pub fn is_valid(&self, now: usize) -> bool {
        !self.invalid() && self.ttl > now
    }
}
