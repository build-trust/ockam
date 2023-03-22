use crate::Message;
use ockam_core::compat::string::String;
use ockam_core::{Address, Route};
use serde::{Deserialize, Serialize};

/// Information about a remotely forwarded worker.
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug, Message)]
pub struct RemoteForwarderInfo {
    pub(crate) forwarding_route: Route,
    pub(crate) remote_address: String,
    pub(crate) worker_address: Address,
}

impl RemoteForwarderInfo {
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
}
