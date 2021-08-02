use ockam_core::lib::fmt::Formatter;
use ockam_core::lib::Display;
use ockam_core::Error;

/// Transport error type
#[derive(Clone, Copy, Debug)]
pub enum TransportError {
    /// None
    None,
    /// Malformed message
    BadMessage,
    /// Failed to send a malformed message
    SendBadMessage,
    /// Failed to receive a malformed message
    RecvBadMessage,
    /// Failed to bind to the desired socket
    BindFailed,
    /// Connection was dropped unexpectedly
    ConnectionDrop,
    /// Connection was already established
    AlreadyConnected,
    /// Connection peer was not found
    PeerNotFound,
    /// Peer requected the incoming connection
    PeerBusy,
    /// Failed to route to an unknown recipient
    UnknownRoute,
    /// Failed to parse the socket address
    InvalidAddress,
    /// A generic I/O failure
    GenericIo,
}

impl TransportError {
    /// Integer code associated with the error domain.
    pub const DOMAIN_CODE: u32 = 100_000;
    /// Error domain
    pub const DOMAIN_NAME: &'static str = "OCKAM_TRANSPORT";
}

impl Display for TransportError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let err: Error = (*self).into();
        err.fmt(f)
    }
}

impl From<TransportError> for Error {
    fn from(e: TransportError) -> Error {
        Error::new(
            TransportError::DOMAIN_CODE + (e as u32),
            TransportError::DOMAIN_NAME,
        )
    }
}

impl From<std::io::Error> for TransportError {
    fn from(e: std::io::Error) -> Self {
        use std::io::ErrorKind::*;
        dbg!();
        match e.kind() {
            ConnectionRefused => Self::PeerNotFound,
            _ => Self::GenericIo,
        }
    }
}

impl<T> From<futures_channel::mpsc::TrySendError<T>> for TransportError {
    fn from(_e: futures_channel::mpsc::TrySendError<T>) -> Self {
        Self::GenericIo
    }
}
