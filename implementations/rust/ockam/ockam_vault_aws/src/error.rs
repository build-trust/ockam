use ockam_core::errcode::{Kind, Origin};
use thiserror::Error;

#[allow(missing_docs)]
#[derive(Error, Debug)]
pub enum Error {
    #[error("aws sdk error creating new key")]
    Create(String),
    #[error("aws sdk error signing message with key {keyid}")]
    Sign { keyid: String, error: String },
    #[error("aws sdk error verifying message with key {keyid}")]
    Verify { keyid: String, error: String },
    #[error("aws sdk error exporting public key {keyid}")]
    Export { keyid: String, error: String },
    #[error("aws sdk error exporting public key {keyid}")]
    Delete { keyid: String, error: String },
    #[error("aws did not return a key id")]
    MissingKeyId,
    #[error("aws did not return the list of existing keys")]
    MissingKeys,
    #[error("aws did not return a signature")]
    MissingSignature,
    #[error("key type is not supported")]
    UnsupportedKeyType,
    #[error("public key der is incorrect")]
    InvalidPublicKeyDer,
    #[error("signature der is incorrect")]
    InvalidSignatureDer,
    #[error("key list was longer than supported")]
    TruncatedKeysList,
    #[error("key was not found")]
    KeyNotFound,
    #[error("invalid handle")]
    InvalidHandle,
}

impl From<Error> for ockam_core::Error {
    fn from(e: Error) -> Self {
        ockam_core::Error::new(Origin::Other, Kind::Io, e)
    }
}
