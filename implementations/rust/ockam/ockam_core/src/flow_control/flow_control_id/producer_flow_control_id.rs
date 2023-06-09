use crate::compat::rand::distributions::{Distribution, Standard};
use crate::compat::rand::Rng;
use crate::compat::string::String;
use crate::flow_control::FlowControlId;
use core::fmt;
use core::fmt::{Debug, Formatter};
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

/// Wrapper around [`FlowControlId`] to guarantee type-safety
#[derive(Clone, Eq, PartialEq, Debug, Ord, PartialOrd, Serialize, Deserialize, Decode, Encode)]
#[rustfmt::skip]
#[cbor(transparent)]
pub struct ProducerFlowControlId(#[n(0)] FlowControlId);

impl Distribution<ProducerFlowControlId> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> ProducerFlowControlId {
        let flow_control_id: FlowControlId = rng.gen();
        ProducerFlowControlId(flow_control_id)
    }
}

impl fmt::Display for ProducerFlowControlId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl From<ProducerFlowControlId> for FlowControlId {
    fn from(value: ProducerFlowControlId) -> Self {
        value.0
    }
}

impl From<String> for ProducerFlowControlId {
    fn from(value: String) -> Self {
        Self(FlowControlId::from(value))
    }
}
