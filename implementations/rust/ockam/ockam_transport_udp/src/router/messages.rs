use ockam_core::{Message, Result};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Serialize, Deserialize, Debug, Message)]
pub enum UdpRouterRequest {
    /// Listen on a local UDP port so the local node can
    /// act as a server to other nodes
    Listen { local_addr: SocketAddr },
}

#[derive(Serialize, Deserialize, Debug, Message)]
pub enum UdpRouterResponse {
    Listen(Result<()>),
}
