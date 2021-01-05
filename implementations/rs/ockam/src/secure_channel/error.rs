use crate::system::commands::OckamCommand;
use ockam_common::error::OckamError;
use std::sync::mpsc::SendError;

/// Represents the failures that can occur in
/// an Ockam SecureChannel
#[derive(Clone, Copy, Debug)]
pub enum Error {
    /// None
    None,
    /// Invalid state
    InvalidState,
    /// Invalid param
    InvalidParam,
    /// Not implemented
    NotImplemented,
    /// Cant send
    CantSend,
    /// Receive error
    RecvError,
}

impl Error {
    /// Error domain
    pub const ERROR_DOMAIN: &'static str = "OCKAM_SECURE_CHANNEL_ERROR_DOMAIN";
}

impl Into<OckamError> for Error {
    fn into(self) -> OckamError {
        OckamError::new(self as u32, Error::ERROR_DOMAIN)
    }
}

impl From<SendError<OckamCommand>> for Error {
    fn from(_: SendError<OckamCommand>) -> Self {
        Error::CantSend.into()
    }
}
