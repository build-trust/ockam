use crate::Message;
use ockam_core::compat::string::String;
use ockam_core::flow_control::FlowControlId;
use ockam_core::{deserialize, serialize, Address, Decodable, Encodable, Encoded, Route};
use serde::{Deserialize, Serialize};

/// Information about a remotely forwarded worker.
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug, Message)]
pub struct RemoteRelayInfo {
    forwarding_route: Route,
    remote_address: String,
    worker_address: Address,
    flow_control_id: Option<FlowControlId>,
}

impl Encodable for RemoteRelayInfo {
    fn encode(self) -> ockam_core::Result<Encoded> {
        serialize(self)
    }
}

impl Decodable for RemoteRelayInfo {
    fn decode(v: &[u8]) -> ockam_core::Result<Self> {
        deserialize(v)
    }
}

impl RemoteRelayInfo {
    /// Constructor
    pub fn new(
        forwarding_route: Route,
        remote_address: String,
        worker_address: Address,
        flow_control_id: Option<FlowControlId>,
    ) -> Self {
        Self {
            forwarding_route,
            remote_address,
            worker_address,
            flow_control_id,
        }
    }
    /// Returns the forwarding route.
    pub fn forwarding_route(&self) -> &Route {
        &self.forwarding_route
    }
    /// Returns the remote address.
    pub fn remote_address(&self) -> &str {
        &self.remote_address
    }
    /// Returns the worker address.
    pub fn worker_address(&self) -> &Address {
        &self.worker_address
    }
    /// Corresponding [`FlowControlId`]
    pub fn flow_control_id(&self) -> &Option<FlowControlId> {
        &self.flow_control_id
    }
}
