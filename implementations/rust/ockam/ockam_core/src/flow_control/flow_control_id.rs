use crate::compat::rand::distributions::{Distribution, Standard};
use crate::compat::rand::Rng;
use crate::compat::string::{String, ToString};
use core::fmt;
use core::fmt::Formatter;
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

/// Unique random identifier of a Flow Control
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct FlowControlId {
    #[n(1)] id: String
}

impl FlowControlId {
    /// Constructor
    fn new(str: &str) -> Self {
        Self {
            id: str.to_string(),
        }
    }
}

impl fmt::Debug for FlowControlId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl fmt::Display for FlowControlId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.id)
    }
}

impl Distribution<FlowControlId> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> FlowControlId {
        let data: [u8; 16] = rng.gen();
        FlowControlId::new(&hex::encode(data))
    }
}

impl From<String> for FlowControlId {
    fn from(value: String) -> Self {
        Self { id: value }
    }
}
