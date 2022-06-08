//! NodeManager request messages

use crate::Message;
use serde::{Deserialize, Serialize};

/// Messaging commands sent to node manager
#[derive(Serialize, Deserialize, Message)]
pub enum NodeManMessage {
    /// Query this node for its status
    Status,
    /// Create a new transport on this node
    CreateTransport {
        /// Encode which type of transport to create
        tt: TransportType,
        /// Specify the runtime mode
        tm: TransportMode,
        /// Transport-specific address payload
        addr: String,
    },
}

/// Encode which type of transport is being requested
#[derive(Serialize, Deserialize, Message)]
pub enum TransportType {
    /// Ockam TCP transport
    Tcp,
    /// Embedded BLE transport
    Ble,
    /// Websocket transport
    WebSocket,
}

/// Encode which type of transport is being requested
#[derive(Serialize, Deserialize, Message)]
pub enum TransportMode {
    /// Listen on a set address
    Listen,
    /// Connect to a remote peer
    Connect,
}
