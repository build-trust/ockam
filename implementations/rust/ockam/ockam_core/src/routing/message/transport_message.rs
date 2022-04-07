use crate::{compat::vec::Vec, Message, Route};
use serde::{Deserialize, Serialize};

/// A generic transport message type.
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
/// See `ockam_transport_tcp::workers::sender::TcpSendWorker` for a usage example.
///
#[derive(Serialize, Deserialize, Debug, Clone, Hash, Ord, PartialOrd, Eq, PartialEq, Message)]
pub struct TransportMessage {
    /// The transport protocol version.
    pub version: u8,
    /// Onward message route.
    pub onward_route: Route,
    /// Return message route.
    ///
    /// This field must be populated by routers handling this message
    /// along the way.
    pub return_route: Route,
    /// The message payload.
    pub payload: Vec<u8>,
}

impl TransportMessage {
    /// Create a new v1 transport message with empty return route.
    pub fn v1(
        onward_route: impl Into<Route>,
        return_route: impl Into<Route>,
        payload: Vec<u8>,
    ) -> Self {
        Self {
            version: 1,
            onward_route: onward_route.into(),
            return_route: return_route.into(),
            payload,
        }
    }
}
