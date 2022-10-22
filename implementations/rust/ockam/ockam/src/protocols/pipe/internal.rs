//! Internal messaging structures

use crate::Message;
use ockam_core::{Decodable, Result, Route, TransportMessage};
use serde::{Deserialize, Serialize};

/// Make the sender re-send a payload
#[derive(Debug, Serialize, Deserialize, Message)]
pub struct Resend {
    /// The index needing to be re-sent
    pub idx: u64,
}

/// Acknowledge successful delivery
#[derive(Debug, Serialize, Deserialize, Message)]
pub struct Ack {
    /// The acknowledged index.
    pub idx: u64,
}

/// Payload sent from handshake listener to newly spawned receiver
#[derive(Debug, Serialize, Deserialize, Message)]
pub struct Handshake {
    /// The route to the sender
    pub route_to_sender: Route,
}

/// An enum containing all internal commands
#[derive(Debug, Serialize, Deserialize, Message)]
pub enum InternalCmd {
    /// Issue the pipe sender to re-send
    Resend(Resend),
    /// Acknowledge receival of pipe message,
    Ack(Ack),
    /// Message received by pipe spawn listener
    InitHandshake,
    /// Message sent from listener to receiver
    Handshake(Handshake),
    /// Initialise a pipe sender with a route
    InitSender,
}

impl InternalCmd {
    /// Decode an [`InternalCmd`] contained inside the given
    /// [`TransportMessage`].
    pub fn from_transport(msg: &TransportMessage) -> Result<Self> {
        Self::decode(&msg.payload)
    }
}
