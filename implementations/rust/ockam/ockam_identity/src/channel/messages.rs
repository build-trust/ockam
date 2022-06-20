use ockam_core::compat::vec::Vec;
use ockam_core::Message;
use serde::{Deserialize, Serialize};

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
    Confirm,
}
