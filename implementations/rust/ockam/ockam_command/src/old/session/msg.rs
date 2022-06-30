use rand::random;
use serde::{Deserialize, Serialize};

use ockam::Message;

#[derive(Serialize, Deserialize, Message, Debug, Clone, PartialEq)]
pub struct RequestId(pub String);

impl RequestId {
    pub fn generate() -> Self {
        let request_id: [u8; 4] = random();
        let request_id = hex::encode(&request_id);

        Self(request_id)
    }
}

impl core::fmt::Display for RequestId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Serialize, Deserialize, Debug, Message, Clone)]
pub enum SessionMsg {
    Heartbeat,
    Ping(RequestId),
    Pong(RequestId),
}
