use ockam_core::Message;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Message)]
pub struct PortalMessage {
    pub binary: Vec<u8>,
}
