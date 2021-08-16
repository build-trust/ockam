use std::array::IntoIter;

use ockam_core::lib::HashMap;
use ockam_transport_core::TransportError;
use tokio_tungstenite::tungstenite::{http::Response, Error as TungsteniteError};

use crate::WebSocketError;

#[test]
fn code_and_domain() {
    let ws_errors_map = IntoIter::new([
        (12, WebSocketError::Transport(TransportError::GenericIo)),
        (0, WebSocketError::Http),
        (1, WebSocketError::Tls),
    ])
    .collect::<HashMap<_, _>>();
    for (expected_code, ws_err) in ws_errors_map {
        let err: ockam_core::Error = ws_err.into();
        match ws_err {
            WebSocketError::Transport(_) => {
                assert_eq!(err.domain(), TransportError::DOMAIN_NAME);
                assert_eq!(err.code(), TransportError::DOMAIN_CODE + expected_code);
            }
            _ => {
                assert_eq!(err.domain(), WebSocketError::DOMAIN_NAME);
                assert_eq!(err.code(), WebSocketError::DOMAIN_CODE + expected_code);
            }
        }
    }
}

#[test]
fn from_tungstenite_error_to_transport_error() {
    let ts_err = TungsteniteError::ConnectionClosed;
    let ws_err: WebSocketError = ts_err.into();
    let err: ockam_core::Error = ws_err.into();
    let expected_err_code = TransportError::ConnectionDrop as u32;
    assert_eq!(err.domain(), TransportError::DOMAIN_NAME);
    assert_eq!(err.code(), TransportError::DOMAIN_CODE + expected_err_code);
}

#[test]
fn from_tungstenite_error_to_websocket_error() {
    let ts_err = TungsteniteError::Http(Response::new(None));
    let ws_err: WebSocketError = ts_err.into();
    let err: ockam_core::Error = ws_err.into();
    let expected_err_code = (WebSocketError::Http).code();
    assert_eq!(err.domain(), WebSocketError::DOMAIN_NAME);
    assert_eq!(err.code(), WebSocketError::DOMAIN_CODE + expected_err_code);
}
