use ockam_core::Message;
use ockam_message_derive::Message;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Message)]
pub enum PortalMessage {
    /// First message that Inlet should send to the Outlet
    Ping,
    /// First message that Outlet sends to the Inlet
    Pong,
    /// Message to indicate that connection from Outlet to the target,
    /// or from the target to the Inlet was dropped
    Disconnect,
    /// Message with binary payload
    Payload(Vec<u8>),
}

#[derive(Serialize, Deserialize, Message)]
pub enum PortalInternalMessage {
    Disconnect,
    /// Message with binary payload
    Payload(Vec<u8>),
}
