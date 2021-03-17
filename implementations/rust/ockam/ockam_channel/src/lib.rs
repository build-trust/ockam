use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum KeyExchangeMessage {
    ChannelMessage(Vec<u8>),
}

pub mod channel;
pub mod channel_factory;
pub mod channels_facade;
pub mod initiator;
pub mod responder;

mod error;
pub use error::*;
