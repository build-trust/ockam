use ockam_core::{
    errcode::{Kind, Origin},
    Error,
};
use ockam_transport_core::TransportError;
use tokio_tungstenite::tungstenite::Error as TungsteniteError;

/// A WebSocket connection worker specific error type.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub(crate) enum WebSocketError {
    /// A wrapped transport error.
    Transport(TransportError),
    /// HTTP error.
    Http,
    /// TLS error.
    Tls,
}
impl ockam_core::compat::error::Error for WebSocketError {}
impl core::fmt::Display for WebSocketError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Transport(t) => write!(f, "ockam transport error {t}"),
            Self::Http => write!(f, "http protocol error"),
            Self::Tls => write!(f, "tls protocol error"),
        }
    }
}

impl From<WebSocketError> for Error {
    #[track_caller]
    fn from(err: WebSocketError) -> Error {
        use WebSocketError::*;
        let kind = match err {
            Transport(_) => Kind::Io,
            Http | Tls => Kind::Protocol,
        };

        Error::new(Origin::Transport, kind, err)
    }
}

impl From<TungsteniteError> for WebSocketError {
    #[track_caller]
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
            TungsteniteError::WriteBufferFull(_) => Self::Transport(TransportError::SendBadMessage),
            TungsteniteError::Http(_) => Self::Http,
            TungsteniteError::Tls(_) => Self::Tls,
            TungsteniteError::AttackAttempt => Self::Transport(TransportError::AttackAttempt),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use ockam_core::compat::collections::HashMap;
    use tokio_tungstenite::tungstenite::http::Response;

    #[test]
    #[ignore]
    fn code_and_domain() {
        let ws_errors_map = [
            (13, WebSocketError::Transport(TransportError::GenericIo)),
            (0, WebSocketError::Http),
            (1, WebSocketError::Tls),
        ]
        .into_iter()
        .collect::<HashMap<_, _>>();
        for (_expected_code, ws_err) in ws_errors_map {
            let _err: Error = ws_err.into();
            if let WebSocketError::Transport(_) = ws_err {
                // assert_eq!(err.code(), TransportError::DOMAIN_CODE + expected_code);
            }
        }
    }

    #[test]
    #[ignore]
    fn from_tungstenite_error_to_transport_error() {
        let ts_err = TungsteniteError::ConnectionClosed;
        let ws_err: WebSocketError = ts_err.into();
        let _err: Error = ws_err.into();
        // let expected_err_code = TransportError::ConnectionDrop as u32;
        // assert_eq!(err.domain(), TransportError::DOMAIN_NAME);
        // assert_eq!(err.code(), TransportError::DOMAIN_CODE + expected_err_code);
    }

    #[test]
    #[ignore]
    fn from_tungstenite_error_to_websocket_error() {
        let ts_err = TungsteniteError::Http(Response::new(None));
        let ws_err: WebSocketError = ts_err.into();
        let _err: Error = ws_err.into();
        // let expected_err_code = (WebSocketError::Http).code();
        // assert_eq!(err.domain(), WebSocketError::DOMAIN_NAME);
        // assert_eq!(err.code(), WebSocketError::DOMAIN_CODE + expected_err_code);
    }
}
