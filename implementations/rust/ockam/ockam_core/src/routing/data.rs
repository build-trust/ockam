use crate::{lib::Vec, Route};
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
    pub onward: Route,
    /// Return message route
    ///
    /// This field must be populated by routers handling this message
    /// along the way.
    pub return_: Route,
    /// The message payload
    pub payload: Vec<u8>,
}

impl TransportMessage {
    /// Create a new v1 transport message with empty return route
    pub fn v1(onward: Route, payload: Vec<u8>) -> Self {
        Self {
            version: 1,
            onward,
            return_: Route::new().into(),
            payload,
        }
    }
}
