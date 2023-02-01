use ockam_core::{Address, Message, Result};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Message)]
pub enum UdsRouterRequest {
    /// Register a new client to this routing scope
    Register {
        /// Specify an accept scope for this client
        accepts: Vec<Address>,
        /// The clients own worker bus address
        self_addr: Address,
    },
    /// Connect to a UDS Peer
    Connect { peer: String },
    /// Disconnect from a UDS Peer
    Disconnect { peer: String },
    /// Unregister (usually, after disconnection)
    Unregister {
        /// The clients own worker bus address
        self_addr: Address,
    },
}

#[derive(Serialize, Deserialize, Debug, Message)]
pub enum UdsRouterResponse {
    /// Response containing a result when attempting to register a new client
    Register(Result<()>),
    /// Response containing an [`Address`] on succesful connection to a peer
    Connect(Result<Address>),
    /// Response containing a result when attempting to disconnect from a peer
    Disconnect(Result<()>),
    /// Resposne containing a result when attempt to unregister
    Unregister(Result<()>),
}
