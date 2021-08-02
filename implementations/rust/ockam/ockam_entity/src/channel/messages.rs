use crate::Contact;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(crate) enum EntityChannelMessage {
    Request { contact: Contact, proof: Vec<u8> },
    Response { contact: Contact, proof: Vec<u8> },
    Confirm,
}
