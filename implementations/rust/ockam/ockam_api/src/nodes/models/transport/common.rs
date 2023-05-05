use minicbor::{Decode, Encode};
#[cfg(feature = "tag")]
use ockam_core::TypeTag;
use ockam_transport_tcp::TcpConnectionMode;
use std::fmt::{self, Display};

/// Encode which type of transport is being requested
// TODO: we have a TransportType in ockam_core.  Do we really want to
// mirror this kind of type here?
#[derive(Copy, Clone, Debug, Decode, Encode, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[rustfmt::skip]
#[cbor(index_only)]
pub enum TransportType {
    /// Ockam TCP transport
    #[n(0)] Tcp,
    /// Embedded BLE transport
    #[n(1)] Ble,
    /// Websocket transport
    #[n(2)] WebSocket,
}

impl Display for TransportType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Tcp => "TCP",
            Self::Ble => "BLE",
            Self::WebSocket => "Websocket",
        })
    }
}

/// Encode which type of transport is being requested
#[derive(Copy, Clone, Debug, Decode, Encode, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[rustfmt::skip]
pub enum TransportMode {
    /// Listen on a set address
    #[n(0)] Listen,
    /// Received a connection from a remote peer
    #[n(1)] Incoming,
    /// Connect to a remote peer
    #[n(2)] Outgoing,
}

impl From<TcpConnectionMode> for TransportMode {
    fn from(value: TcpConnectionMode) -> Self {
        match value {
            TcpConnectionMode::Outgoing => Self::Outgoing,
            TcpConnectionMode::Incoming => Self::Incoming,
        }
    }
}

impl Display for TransportMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Listen => "Listening",
            Self::Incoming => "Incoming connection",
            Self::Outgoing => "Outgoing connection",
        })
    }
}
