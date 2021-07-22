use crate::lib::Vec;
use crate::{Address, LocalMessage};
use serde::{Deserialize, Serialize};

/// A command message for router implementations
///
/// If a router is implemented as a worker, it should accept this
/// message type.
#[derive(Serialize, Deserialize, Debug, Clone, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub enum RouterMessage {
    /// Route the provided message towards its destination
    Route(LocalMessage),
    /// Register a new client to this routing scope
    Register {
        /// Specify an accept scope for this client
        accepts: Vec<Address>,
        /// The clients own worker bus address
        self_addr: Address,
    },
}
