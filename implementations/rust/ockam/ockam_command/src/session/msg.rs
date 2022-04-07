use ockam::Message;
use rand::random;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Message, Clone, PartialEq)]
pub struct RequestId(pub String);

impl RequestId {
    pub fn generate() -> Self {
        let request_id: [u8; 4] = random();
        let request_id = hex::encode(&request_id);

        Self(request_id)
    }
}

#[derive(Serialize, Deserialize, Message, Clone)]
pub enum SessionMsg {
    Heartbeat,
    Ping(RequestId),
    Pong(RequestId),
}
