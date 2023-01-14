use ockam_core::compat::vec::Vec;
use ockam_core::Message;
use serde::{Deserialize, Serialize};

// Could be one struct, but backwards compatibility...
#[derive(Serialize, Deserialize, Message)]
pub(crate) enum IdentityChannelMessage {
    Request {
        identity: Vec<u8>,
        signature: Vec<u8>,
    },
    Response {
        identity: Vec<u8>,
        signature: Vec<u8>,
    },
}

impl IdentityChannelMessage {
    pub fn consume(self) -> (Vec<u8>, Vec<u8>) {
        match self {
            IdentityChannelMessage::Request {
                identity,
                signature,
            } => (identity, signature),
            IdentityChannelMessage::Response {
                identity,
                signature,
            } => (identity, signature),
        }
    }
}
