use ockam_core::compat::vec::Vec;
use ockam_core::Error;
use ockam_core::Message;
use serde::{Deserialize, Serialize};

/// Request type for `EncryptorWorker` API Address
#[derive(Serialize, Deserialize, Message)]
pub struct EncryptionRequest(pub Vec<u8>);

/// Response type for `EncryptorWorker` API Address
#[derive(Serialize, Deserialize, Message)]
pub enum EncryptionResponse {
    /// Success
    Ok(Vec<u8>),
    /// Error
    Err(Error),
}

/// Request type for `DecryptorWorker` API Address
#[derive(Serialize, Deserialize, Message)]
pub struct DecryptionRequest(pub Vec<u8>);

/// Response type for `DecryptorWorker` API Address
#[derive(Serialize, Deserialize, Message)]
pub enum DecryptionResponse {
    /// Success
    Ok(Vec<u8>),
    /// Error
    Err(Error),
}
