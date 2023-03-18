use std::net::SocketAddr;

use ockam_core::{Message, Result};
use serde::{Deserialize, Serialize};

/// Request type for UDP Hole Punching Rendezvous service
#[derive(Serialize, Deserialize, Debug, Message)]
pub enum RendezvousRequest {
    Update {
        /// Name of sending node
        node_name: String,
    },
    Query {
        /// Name of node to lookup
        node_name: String,
    },
}

/// Response type for UDP Hole Punching Rendezvous service
#[derive(Serialize, Deserialize, Debug, Message)]
pub enum RendezvousResponse {
    Update(Result<()>),
    Query(Result<SocketAddr>),
}
