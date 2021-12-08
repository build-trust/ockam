use crate::Message;
use ockam_core::Address;
use serde::{Deserialize, Serialize};

/// A simple message type to create a bi-directional channel
#[derive(Debug, Serialize, Deserialize, Message)]
pub struct ChannelCreationHandshake(pub Address, pub Address);
