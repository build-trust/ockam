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
pub struct SpawnerFlowControlId(#[n(0)]FlowControlId);

impl Distribution<SpawnerFlowControlId> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> SpawnerFlowControlId {
        let flow_control_id: FlowControlId = rng.gen();
        SpawnerFlowControlId(flow_control_id)
    }
}

impl fmt::Display for SpawnerFlowControlId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl From<SpawnerFlowControlId> for FlowControlId {
    fn from(value: SpawnerFlowControlId) -> Self {
        value.0
    }
}

impl From<String> for SpawnerFlowControlId {
    fn from(value: String) -> Self {
        Self(FlowControlId::from(value))
    }
}
