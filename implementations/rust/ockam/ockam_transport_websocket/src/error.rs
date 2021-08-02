use std::fmt::{Display, Formatter};

use crate::common::TransportError;
use ockam_core::Error;

/// WebSocket Transport specific error type
#[derive(Clone, Copy, Debug)]
pub enum WebSocketError {
    /// A generic websocket failure
    WebSocket,
}

impl WebSocketError {
    /// Integer code associated with the error domain.
    pub const DOMAIN_CODE: u32 = 21_000;
    /// Error domain
    pub const DOMAIN_NAME: &'static str = "OCKAM_TRANSPORT_WEBSOCKET";
}

impl Display for WebSocketError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let err: Error = (*self).into();
        err.fmt(f)
    }
}

impl From<WebSocketError> for Error {
    fn from(e: WebSocketError) -> Error {
        Error::new(
            WebSocketError::DOMAIN_CODE + (e as u32),
            WebSocketError::DOMAIN_NAME,
        )
    }
}

impl From<tokio_tungstenite::tungstenite::Error> for TransportError {
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
            _ => Self::None,
        }
    }
}
