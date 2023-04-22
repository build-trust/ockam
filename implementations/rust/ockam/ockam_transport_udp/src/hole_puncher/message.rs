use ockam_core::Message;
use serde::{Deserialize, Serialize};

// TODO: Use CBOR encoding for messages

/// Internal message type for UDP NAT Hole Puncher
#[derive(Serialize, Deserialize, Debug, Message, Clone)]
pub(crate) enum PunchMessage {
    Ping,
    Pong,
    Heartbeat,
    WaitForHoleOpen,
    Payload(Vec<u8>),
}
