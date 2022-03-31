//! Ockam channel protocol structures
use crate::Message;
use ockam_core::Address;
use serde::{Deserialize, Serialize};

/// A simple message type to create a bi-directional channel
#[derive(Debug, Serialize, Deserialize, Message)]
pub struct ChannelCreationHandshake {
    pub(crate) channel_addr: Address,
    pub(crate) tx_addr: Address,
    pub(crate) tx_int_addr: Address,
    pub(crate) rx_addr: Address,
    pub(crate) rx_int_addr: Address,
}
