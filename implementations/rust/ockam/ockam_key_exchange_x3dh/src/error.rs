use ockam_core::{
    errcode::{Kind, Origin},
    Error,
};

/// Represents the failures that can occur in
/// an Ockam X3DH kex
#[derive(Clone, Copy, Debug)]
pub enum X3DHError {
    InvalidState = 1,
    MessageLenMismatch,
    SignatureLenMismatch,
    InvalidHash,
}

impl ockam_core::compat::error::Error for X3DHError {}
impl core::fmt::Display for X3DHError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidState => "invalid state".fmt(f),
            Self::MessageLenMismatch => "message length mismatch".fmt(f),
            Self::SignatureLenMismatch => "signature length mismatch".fmt(f),
            Self::InvalidHash => "invalid hash".fmt(f),
        }
    }
}

impl From<X3DHError> for Error {
    fn from(err: X3DHError) -> Self {
        use X3DHError::*;
        let kind = match err {
            InvalidState | InvalidHash => Kind::Invalid,
            MessageLenMismatch | SignatureLenMismatch => Kind::Misuse,
        };

        Error::new(Origin::KeyExchange, kind, err)
    }
}
