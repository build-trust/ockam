use crate::compat::rand::distributions::{Distribution, Standard};
use crate::compat::rand::Rng;
use crate::compat::string::{String, ToString};
use serde::{Deserialize, Serialize};

/// Unique random identifier of a Flow Control
#[derive(Clone, Eq, PartialEq, Debug, Ord, PartialOrd, Serialize, Deserialize)]
pub struct FlowControlId(String);

impl FlowControlId {
    /// Constructor
    pub fn new(str: &str) -> Self {
        Self(str.to_string())
    }
}

impl ToString for FlowControlId {
    fn to_string(&self) -> String {
        self.0.clone()
    }
}

impl From<&str> for FlowControlId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl Distribution<FlowControlId> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> FlowControlId {
        let address: [u8; 16] = rng.gen();
        FlowControlId(hex::encode(address))
    }
}
