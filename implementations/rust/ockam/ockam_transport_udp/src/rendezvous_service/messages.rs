use ockam_core::{Message, Result, Route};
use serde::{Deserialize, Serialize};

// TODO: Change this Request/Response protocol to use CBOR encoding for messages.

/// Request type for UDP Hole Punching Rendezvous service
#[derive(Serialize, Deserialize, Debug, Message)]
pub enum RendezvousRequest {
    /// Update service's internal table with the
    /// details of the sending node.
    Update {
        /// Name of sending node's puncher
        puncher_name: String,
    },
    /// Query service's internal table for the public
    /// route to the named node.
    Query {
        /// Name of puncher to lookup
        puncher_name: String,
    },
    /// Ping service to see if it is reachable and working.
    Ping,
}

/// Response type for UDP Hole Punching Rendezvous service
#[derive(Serialize, Deserialize, Debug, Message)]
pub enum RendezvousResponse {
    Query(Result<Route>),
    Pong,
}
