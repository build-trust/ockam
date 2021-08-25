use crate::Contact;
use ockam_core::compat::vec::Vec;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(crate) enum EntityChannelMessage {
    Request { contact: Contact, proof: Vec<u8> },
    Response { contact: Contact, proof: Vec<u8> },
    Confirm,
}
