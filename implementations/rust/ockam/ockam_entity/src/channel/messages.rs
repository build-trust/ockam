use crate::Contact;
use ockam_core::compat::vec::Vec;
use ockam_core::Message;
use ockam_message_derive::Message;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Message)]
pub(crate) enum EntityChannelMessage {
    Request { contact: Contact, proof: Vec<u8> },
    Response { contact: Contact, proof: Vec<u8> },
    Confirm,
}
