use crate::{lib::Vec, Address, Route};
use serde::{Deserialize, Serialize};

/// A generic transport message
///
/// While this type is exposed in ockam_core (and the root `ockam`
/// crate) in order to provide a mechanism for third-party developers
/// to create custom transport channel routers.  Casual users of ockam
/// should never have to interact with this type directly.
#[derive(Serialize, Deserialize, Debug, Clone, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub struct TransportMessage {
    /// The transport protocol version
    pub version: u8,
    /// Onward message route
    pub onward_route: Route,
    /// Return message route
    ///
    /// This field must be populated by routers handling this message
    /// along the way.
    pub return_route: Route,
    /// The message payload
    pub payload: Vec<u8>,
}

impl TransportMessage {
    /// Create a new v1 transport message with empty return route
    pub fn v1(onward_route: Route, payload: Vec<u8>) -> Self {
        Self {
            version: 1,
            onward_route,
            return_route: Route::new().into(),
            payload,
        }
    }
}

/// A command message for router implementations
///
/// If a router is implemented as a worker, it should accept this
/// message type.
#[derive(Serialize, Deserialize, Debug, Clone, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub enum RouterMessage {
    /// Route the provided message towards its destination
    Route(TransportMessage),
    /// Register a new client to this routing scope
    Register {
        /// Specify an accept scope for this client
        accepts: Address,
        /// The clients own worker bus address
        self_addr: Address,
    },
}
