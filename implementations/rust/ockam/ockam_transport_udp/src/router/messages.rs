use ockam_core::{Address, Message, Result};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Message)]
pub enum UdpRouterRequest {
    /// Register a new client to this routing scope.
    Register {
        /// Specify an accept scope for this client.
        accepts: Vec<Address>,
        /// The clients own worker bus address.
        self_addr: Address,
    },
}

#[derive(Serialize, Deserialize, Debug, Message)]
pub enum UdpRouterResponse {
    Register(Result<()>),
}
