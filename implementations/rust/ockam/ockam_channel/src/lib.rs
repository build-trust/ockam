//use serde::{Deserialize, Serialize};

// ToDo - this should be the message sent by the key exchanger to its parent.
// But CompletedKeyExchange doesn't implement Clone, Serialize, and Deserialize.
// And it's unclear how to define the message type that "app" receives.
// #[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
// pub enum ExchangerMessage {
//     ExchangeComplete(CompletedKeyExchange),
// }

pub mod initiator;
pub mod responder;

mod error;
pub use error::*;
//use ockam_key_exchange_core::CompletedKeyExchange;
