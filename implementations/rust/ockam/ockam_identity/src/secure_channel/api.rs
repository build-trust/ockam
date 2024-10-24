use ockam_core::compat::vec::Vec;
use ockam_core::Error;
use ockam_core::Message;
use serde::{Deserialize, Serialize};

/// Request type for `EncryptorWorker` API Address
#[derive(Serialize, Deserialize, Message)]
pub enum EncryptionRequest {
    /// Encrypt data
    Encrypt(Vec<u8>),
    /// Trigger a manual rekey
    Rekey,
}

/// Response type for `EncryptorWorker` API Address
#[derive(Serialize, Deserialize, Message)]
pub enum EncryptionResponse {
    /// Success
    Ok(Vec<u8>),
    /// Error
    Err(Error),
}

/// Request type for `Decryptor` API Address (the `Decryptor` is accessible through the `HandshakeWorker`)
#[derive(Serialize, Deserialize, Message)]
pub struct DecryptionRequest(pub Vec<u8>, pub Option<u16>);

/// Response type for `Decryptor` API Address (the `Decryptor` is accessible through the `HandshakeWorker`)
#[derive(Serialize, Deserialize, Message)]
pub enum DecryptionResponse {
    /// Success
    Ok(Vec<u8>),
    /// Error
    Err(Error),
}
