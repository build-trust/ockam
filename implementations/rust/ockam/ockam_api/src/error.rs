use core::fmt;
use miette::Diagnostic;

use ockam_core::errcode::{Kind, Origin};

/// Potential API errors
#[derive(Debug, thiserror::Error, Diagnostic)]
pub enum ApiError {
    #[error(transparent)]
    Core(#[from] ockam_core::Error),

    #[error(transparent)]
    MultiAddr(#[from] ockam_multiaddr::Error),

    #[error(transparent)]
    Parse(#[from] ParseError),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
}

#[derive(Debug, thiserror::Error, Diagnostic)]
pub enum ParseError {
    #[error(transparent)]
    Addr(#[from] std::net::AddrParseError),

    #[error(transparent)]
    Url(#[from] url::ParseError),

    #[error(transparent)]
    CborDecode(#[from] minicbor::decode::Error),

    #[error(transparent)]
    CborEncode(#[from] minicbor::encode::Error<std::io::Error>),

    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),
}

impl ApiError {
    pub fn message<T: fmt::Display>(m: T) -> ApiError {
        ockam_core::Error::new(Origin::Application, Kind::Unknown, m.to_string()).into()
    }

    pub fn core<T: fmt::Display>(m: T) -> ockam_core::Error {
        ockam_core::Error::new(Origin::Application, Kind::Unknown, m.to_string())
    }
}
