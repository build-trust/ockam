use crate::compat::rand::distributions::{Distribution, Standard};
use crate::compat::rand::Rng;
use crate::compat::string::{String, ToString};
#[cfg(feature = "tag")]
use crate::TypeTag;
use core::fmt;
use core::fmt::Formatter;
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

/// Unique random identifier of a Flow Control
#[derive(Clone, Eq, PartialEq, Debug, Ord, PartialOrd, Serialize, Deserialize, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct FlowControlId {
    #[cfg(feature = "tag")]
    #[serde(skip)]
    #[n(0)] tag: TypeTag<6020561>,
    #[n(1)] id: String
}

impl FlowControlId {
    /// Constructor
    fn new(str: &str) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            id: str.to_string(),
        }
    }
}
impl fmt::Display for FlowControlId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.id)
    }
}

impl Distribution<FlowControlId> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> FlowControlId {
        let address: [u8; 16] = rng.gen();
        FlowControlId::new(&hex::encode(address))
    }
}
