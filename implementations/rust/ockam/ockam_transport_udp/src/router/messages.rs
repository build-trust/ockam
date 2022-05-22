use ockam_core::{Address, Message};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Hash, Ord, PartialOrd, Eq, PartialEq, Message)]
pub(crate) enum UdpRouterMessage {
    /// Register a new client to this routing scope.
    Register {
        /// Specify an accept scope for this client.
        accepts: Vec<Address>,
        /// The clients own worker bus address.
        self_addr: Address,
    },
}
