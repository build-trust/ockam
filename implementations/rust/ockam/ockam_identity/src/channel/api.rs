use ockam_core::compat::vec::Vec;
use ockam_core::Error;
use ockam_core::Message;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Message)]
pub struct EncryptionRequest(pub Vec<u8>);

#[derive(Serialize, Deserialize, Message)]
pub enum EncryptionResponse {
    Ok(Vec<u8>),
    Err(Error),
}

#[derive(Serialize, Deserialize, Message)]
pub struct DecryptionRequest(pub Vec<u8>);

#[derive(Serialize, Deserialize, Message)]
pub enum DecryptionResponse {
    Ok(Vec<u8>),
    Err(Error),
}
