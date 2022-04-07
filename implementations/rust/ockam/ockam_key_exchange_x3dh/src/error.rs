use ockam_core::{
    errcode::{ErrorCode, Kind, Origin},
    thiserror, Error,
};

/// Represents the failures that can occur in
/// an Ockam X3DH kex
#[derive(Clone, Copy, Debug, thiserror::Error)]
pub enum X3DHError {
    #[error("invalid state")]
    InvalidState = 1,
    #[error("message length mismatch")]
    MessageLenMismatch,
    #[error("signature length mismatch")]
    SignatureLenMismatch,
    #[error("invalid hash")]
    InvalidHash,
}

impl From<X3DHError> for Error {
    fn from(err: X3DHError) -> Self {
        use X3DHError::*;
        let kind = match err {
            InvalidState | InvalidHash => Kind::Invalid,
            MessageLenMismatch | SignatureLenMismatch => Kind::Misuse,
        };

        Error::new(ErrorCode::new(Origin::KeyExchange, kind), err)
    }
}
