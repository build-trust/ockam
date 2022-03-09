use std::fmt::{Display, Formatter};

use tokio_tungstenite::tungstenite::Error as TungsteniteError;

use ockam_core::Error;
use ockam_transport_core::TransportError;

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
        let info = format!("{}::{:?}", module_path!(), e);
        match e {
            WebSocketError::Transport(e) => e.into(),
            e => Error::new(WebSocketError::DOMAIN_CODE + e.code(), info),
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

impl From<futures_channel::mpsc::SendError> for WebSocketError {
    fn from(_e: futures_channel::mpsc::SendError) -> Self {
        Self::Transport(TransportError::GenericIo)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use tokio_tungstenite::tungstenite::http::Response;

    #[test]
    fn code_and_domain() {
        let ws_errors_map = [
            (13, WebSocketError::Transport(TransportError::GenericIo)),
            (0, WebSocketError::Http),
            (1, WebSocketError::Tls),
        ]
        .into_iter();
        for (expected_code, ws_err) in ws_errors_map {
            let err: Error = ws_err.into();
            match ws_err {
                WebSocketError::Transport(_) => {
                    assert_eq!(err.code(), TransportError::DOMAIN_CODE + expected_code);
                }
                _ => {
                    assert_eq!(err.code(), WebSocketError::DOMAIN_CODE + expected_code);
                }
            }
        }
    }

    #[test]
    fn from_tungstenite_error_to_transport_error() {
        let ts_err = TungsteniteError::ConnectionClosed;
        let ws_err: WebSocketError = ts_err.into();
        let err: Error = ws_err.into();
        let expected_err_code = TransportError::ConnectionDrop as u32;
        assert_eq!(err.code(), TransportError::DOMAIN_CODE + expected_err_code);
    }

    #[test]
    fn from_tungstenite_error_to_websocket_error() {
        let ts_err = TungsteniteError::Http(Response::new(None));
        let ws_err: WebSocketError = ts_err.into();
        let err: Error = ws_err.into();
        let expected_err_code = (WebSocketError::Http).code();
        assert_eq!(err.code(), WebSocketError::DOMAIN_CODE + expected_err_code);
    }
}
