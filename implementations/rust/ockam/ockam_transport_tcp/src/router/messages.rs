use ockam_core::{Address, Message, Result};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Message)]
pub enum TcpRouterRequest {
    /// Register a new client to this routing scope.
    Register {
        /// Specify an accept scope for this client.
        accepts: Vec<Address>,
        /// The clients own worker bus address.
        self_addr: Address,
    },
    /// Connect
    Connect { peer: String },
    /// Unregister (usually, after disconnection)
    Unregister {
        /// The clients own worker bus address.
        self_addr: Address,
    },
}

#[derive(Serialize, Deserialize, Debug, Message)]
pub enum TcpRouterResponse {
    Register(Result<()>),
    Connect(Result<Address>),
    Unregister(Result<()>),
}
