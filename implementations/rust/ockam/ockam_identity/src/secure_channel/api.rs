use ockam_core::compat::vec::Vec;
use ockam_core::Message;
use ockam_core::{deserialize, serialize, Decodable, Encodable, Encoded, Error};
use serde::{Deserialize, Serialize};

/// Request type for `EncryptorWorker` API Address
#[derive(Serialize, Deserialize, Message)]
pub struct EncryptionRequest(pub Vec<u8>);

impl Encodable for EncryptionRequest {
    fn encode(self) -> ockam_core::Result<Encoded> {
        Encodable::encode(self.0)
    }
}

impl Decodable for EncryptionRequest {
    fn decode(v: &[u8]) -> ockam_core::Result<Self> {
        Ok(EncryptionRequest(Decodable::decode(v)?))
    }
}

/// Response type for `EncryptorWorker` API Address
#[derive(Serialize, Deserialize, Message)]
pub enum EncryptionResponse {
    /// Success
    Ok(Vec<u8>),
    /// Error
    Err(Error),
}

impl Encodable for EncryptionResponse {
    fn encode(self) -> ockam_core::Result<Encoded> {
        serialize(self)
    }
}

impl Decodable for EncryptionResponse {
    fn decode(v: &[u8]) -> ockam_core::Result<Self> {
        deserialize(v)
    }
}
/// Request type for `Decryptor` API Address (the `Decryptor` is accessible through the `HandshakeWorker`)
#[derive(Serialize, Deserialize, Message)]
pub struct DecryptionRequest(pub Vec<u8>);

impl Encodable for DecryptionRequest {
    fn encode(self) -> ockam_core::Result<Encoded> {
        Encodable::encode(self.0)
    }
}

impl Decodable for DecryptionRequest {
    fn decode(v: &[u8]) -> ockam_core::Result<Self> {
        Ok(DecryptionRequest(Decodable::decode(v)?))
    }
}
/// Response type for `Decryptor` API Address (the `Decryptor` is accessible through the `HandshakeWorker`)
#[derive(Serialize, Deserialize, Message)]
pub enum DecryptionResponse {
    /// Success
    Ok(Vec<u8>),
    /// Error
    Err(Error),
}

impl Encodable for DecryptionResponse {
    fn encode(self) -> ockam_core::Result<Encoded> {
        serialize(self)
    }
}

impl Decodable for DecryptionResponse {
    fn decode(v: &[u8]) -> ockam_core::Result<Self> {
        deserialize(v)
    }
}
