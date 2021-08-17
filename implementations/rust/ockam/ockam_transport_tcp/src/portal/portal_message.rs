use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct PortalMessage {
    pub binary: Vec<u8>,
}
