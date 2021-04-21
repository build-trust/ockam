use crate::init::WorkerPair;
use ockam::Error;

/// A WebSocket connection worker specific error type
#[derive(Clone, Copy, Debug)]
pub enum WebSocketError {
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
    /// A generic websocket failure
    WebSocket,
}

impl WebSocketError {
    /// Integer code associated with the error domain.
    pub const DOMAIN_CODE: u32 = 15_000; //TODO: what code should we use?
    /// Error domain
    pub const DOMAIN_NAME: &'static str = "OCKAM_TRANSPORT_WEBSOCKET";
}

impl From<WebSocketError> for Error {
    fn from(e: WebSocketError) -> Error {
        Error::new(
            WebSocketError::DOMAIN_CODE + (e as u32),
            WebSocketError::DOMAIN_NAME,
        )
    }
}

impl From<std::io::Error> for WebSocketError {
    fn from(e: std::io::Error) -> Self {
        use std::io::ErrorKind::*;
        dbg!();
        match e.kind() {
            ConnectionRefused => Self::PeerNotFound,
            _ => Self::GenericIo,
        }
    }
}

impl From<tokio_tungstenite::tungstenite::Error> for WebSocketError {
    fn from(e: tokio_tungstenite::tungstenite::Error) -> Self {
        use tokio_tungstenite::tungstenite::Error as TungsteniteError;
        match e {
            TungsteniteError::ConnectionClosed | TungsteniteError::AlreadyClosed => {
                Self::ConnectionDrop
            }
            TungsteniteError::Io(_) => Self::GenericIo,
            TungsteniteError::Url(_)
            | TungsteniteError::Http(_)
            | TungsteniteError::HttpFormat(_) => Self::InvalidAddress,
            TungsteniteError::Capacity(_) | TungsteniteError::Utf8 => Self::BadMessage,
            _ => Self::WebSocket,
        }
    }
}

impl From<futures_channel::mpsc::TrySendError<WorkerPair>> for WebSocketError {
    fn from(_e: futures_channel::mpsc::TrySendError<WorkerPair>) -> Self {
        Self::GenericIo
    }
}
