use ockam_core::Message;
use serde::{Deserialize, Serialize};

/// A command message type for a Portal
#[derive(Serialize, Deserialize, Message, Debug)]
pub enum PortalMessage {
    /// First message that Inlet sends to the Outlet
    Ping,
    /// First message that Outlet sends to the Inlet
    Pong,
    /// Message to indicate that connection from Outlet to the target,
    /// or from the target to the Inlet was dropped
    Disconnect,
    /// Message with binary payload
    Payload(Vec<u8>),
}

/// An internal message type for a Portal
#[derive(Serialize, Deserialize, Message)]
pub enum PortalInternalMessage {
    /// Connection was dropped
    Disconnect,
}

///Maximum allowed size for a payload
pub const MAX_PAYLOAD_SIZE: usize = 48 * 1024;
