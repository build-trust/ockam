use std::fmt::{Display, Formatter};

use ockam_core::Error;
use ockam_transport_core::TransportError;
use tokio_tungstenite::tungstenite::Error as TungsteniteError;

/// A WebSocket connection worker specific error type
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub enum WebSocketError {
    /// A wrapped transport error
    Transport(TransportError),
    /// HTTP error
    Http,
    /// TLS error
    Tls,
}

impl WebSocketError {
    /// Integer code associated with the error domain.
    pub const DOMAIN_CODE: u32 = 21_000;
    /// Error domain
    pub const DOMAIN_NAME: &'static str = "OCKAM_TRANSPORT_WEBSOCKET";

    pub fn code(&self) -> u32 {
        match self {
            WebSocketError::Transport(_) => 0,
            WebSocketError::Http => 0,
            WebSocketError::Tls => 1,
        }
    }
}

impl Display for WebSocketError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let err: Error = (*self).into();
        err.fmt(f)
    }
}

impl From<WebSocketError> for Error {
    fn from(e: WebSocketError) -> Error {
        match e {
            WebSocketError::Transport(e) => e.into(),
            _ => Error::new(
                WebSocketError::DOMAIN_CODE + e.code(),
                WebSocketError::DOMAIN_NAME,
            ),
        }
    }
}

impl From<TungsteniteError> for WebSocketError {
    fn from(e: TungsteniteError) -> Self {
        match e {
            TungsteniteError::ConnectionClosed => Self::Transport(TransportError::ConnectionDrop),
            TungsteniteError::AlreadyClosed => Self::Transport(TransportError::ConnectionDrop),
            TungsteniteError::Io(_) => Self::Transport(TransportError::GenericIo),
            TungsteniteError::Url(_) => Self::Transport(TransportError::InvalidAddress),
            TungsteniteError::HttpFormat(_) => Self::Transport(TransportError::InvalidAddress),
            TungsteniteError::Capacity(_) => Self::Transport(TransportError::Capacity),
            TungsteniteError::Utf8 => Self::Transport(TransportError::Encoding),
            TungsteniteError::Protocol(_) => Self::Transport(TransportError::Protocol),
            TungsteniteError::SendQueueFull(_) => Self::Transport(TransportError::SendBadMessage),
            TungsteniteError::Http(_) => Self::Http,
            TungsteniteError::Tls(_) => Self::Tls,
        }
    }
}

impl<T> From<futures_channel::mpsc::TrySendError<T>> for WebSocketError {
    fn from(_e: futures_channel::mpsc::TrySendError<T>) -> Self {
        Self::Transport(TransportError::GenericIo)
    }
}
