use crate::Message;
use ockam_core::Address;
use serde::{Deserialize, Serialize};

/// A simple message type to create a bi-directional channel
#[derive(Debug, Serialize, Deserialize, Message)]
pub struct ChannelCreationHandshake {
    pub channel_addr: Address,
    pub tx_addr: Address,
    pub tx_int_addr: Address,
    pub rx_addr: Address,
    pub rx_int_addr: Address,
}
