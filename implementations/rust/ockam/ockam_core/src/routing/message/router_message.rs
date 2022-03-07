use crate::compat::vec::Vec;
use crate::{Address, LocalMessage, Message};
use serde::{Deserialize, Serialize};

/// The command message type for router implementations.
///
/// If a router is implemented as a worker, it should accept this
/// message type.
///
/// This type is exposed in `ockam_core` (and the root `ockam` crate) in
/// order to provide a mechanism for third-party developers to create
/// custom transport channel routers.
///
/// Casual users of Ockam should never have to interact with this type
/// directly.
///
/// # Examples
///
/// See `ockam_transport_tcp::router::TcpRouter` for a usage example.
///
#[derive(Serialize, Deserialize, Debug, Clone, Hash, Ord, PartialOrd, Eq, PartialEq, Message)]
pub enum RouterMessage {
    /// Route the provided message towards its destination.
    Route(LocalMessage),
    /// Register a new client to this routing scope.
    Register {
        /// Specify an accept scope for this client.
        accepts: Vec<Address>,
        /// The clients own worker bus address.
        self_addr: Address,
    },
}
