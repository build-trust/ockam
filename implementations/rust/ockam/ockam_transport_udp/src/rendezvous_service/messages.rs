use ockam_core::{Message, Result, Route};
use serde::{Deserialize, Serialize};

/// Request type for UDP Hole Punching Rendezvous service
#[derive(Serialize, Deserialize, Debug, Message)]
pub enum RendezvousRequest {
    /// Update service's internal table with the
    /// return route of the sending node.
    Update {
        /// Name of sending node
        node_name: String,
    },
    /// Query service's internal table for the public
    /// route to the named node.
    Query {
        /// Name of node to lookup
        node_name: String,
    },
    /// Ping service to see if it is reachable and working.
    Ping,
}

/// Response type for UDP Hole Punching Rendezvous service
#[derive(Serialize, Deserialize, Debug, Message)]
pub enum RendezvousResponse {
    Update(Result<Route>),
    Query(Result<Route>),
    Pong,
}
