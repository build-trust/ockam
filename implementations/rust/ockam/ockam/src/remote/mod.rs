//! [`RemoteForwarder`] allows registering node within a Cloud Node with dynamic or static alias,
//! which allows other nodes forward messages to local workers on this node using that alias.

mod addresses;
mod info;
mod lifecycle;
mod options;
mod worker;

pub use info::*;
pub use options::*;

use crate::remote::addresses::Addresses;
use core::time::Duration;
use ockam_core::compat::{string::String, vec::Vec};
use ockam_core::flow_control::FlowControlId;
use ockam_core::Route;
use ockam_node::DelayedEvent;

/// This Worker is responsible for registering on Ockam Orchestrator and forwarding messages to local Worker
pub struct RemoteForwarder {
    /// Address used from other node
    addresses: Addresses,
    completion_msg_sent: bool,
    registration_route: Route,
    registration_payload: String,
    flow_control_id: Option<FlowControlId>,
    // We only use Heartbeat for static RemoteForwarder
    heartbeat: Option<DelayedEvent<Vec<u8>>>,
    heartbeat_interval: Duration,
}
