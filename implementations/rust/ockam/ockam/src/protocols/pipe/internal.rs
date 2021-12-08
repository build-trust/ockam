//! Internal messaging structures

use crate::Message;
use ockam_core::{Decodable, Result, Route, TransportMessage};
use serde::{Deserialize, Serialize};

/// Make the sender re-send a payload
#[derive(Debug, Serialize, Deserialize, Message)]
pub struct Resend {
    pub idx: u64,
}

/// Acknowlege successful delivery
#[derive(Debug, Serialize, Deserialize, Message)]
pub struct Ack {
    pub idx: u64,
}

/// Payload sent from handshake listener to newly spawned receiver
#[derive(Debug, Serialize, Deserialize, Message)]
pub struct Handshake {
    pub route_to_sender: Route,
}

/// An enum containing all internal commands
#[derive(Debug, Serialize, Deserialize, Message)]
pub enum InternalCmd {
    /// Issue the pipe sender to re-send
    Resend(Resend),
    /// Acknowlege receival of pipe message,
    Ack(Ack),
    /// Message received by pipe spawn listener
    InitHandshake,
    /// Message sent from listener to receiver
    Handshake(Handshake),
    /// Initialise a pipe sender with a route
    InitSender,
}

impl InternalCmd {
    pub fn from_transport(msg: &TransportMessage) -> Result<Self> {
        Self::decode(&msg.payload)
    }
}
