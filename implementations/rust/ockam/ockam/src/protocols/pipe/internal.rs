//! Internal messaging structures

use crate::Message;
use ockam_core::Route;
use serde::{Deserialize, Serialize};

/// Internal command issued to
#[derive(Serialize, Deserialize, Message)]
pub struct CreatePipe {
    pub route_to_sender: Route,
}

/// Make the sender re-send a payload
#[derive(Serialize, Deserialize, Message)]
pub struct Resend {
    pub idx: u64,
}

/// Acknowlege successful delivery
#[derive(Serialize, Deserialize, Message)]
pub struct Ack {
    pub idx: u64,
}

/// An enum containing all internal commands
#[derive(Serialize, Deserialize, Message)]
pub enum InternalCmd {
    /// A create_pipe handshake message
    Create(CreatePipe),
    /// Issue the pipe sender to re-send
    ///
    /// This command must be ignored on PipeReceiver
    Resend(Resend),
    /// Acknowlege receival of pipe message,
    Ack(Ack),
}
