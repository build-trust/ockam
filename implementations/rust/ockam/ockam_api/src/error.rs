use crate::cli_state::CliStateError;
use core::fmt;
use miette::Diagnostic;
use ockam_core::errcode::{Kind, Origin};

pub type Result<T> = core::result::Result<T, ApiError>;

/// Potential API errors
#[derive(Debug, thiserror::Error, Diagnostic)]
pub enum ApiError {
    #[error("{0}")]
    General(String),

    #[error(transparent)]
    Core(#[from] ockam_core::Error),

    #[error(transparent)]
    MultiAddr(#[from] ockam_multiaddr::Error),

    #[error(transparent)]
    Parse(#[from] ParseError),

    #[error(transparent)]
    Ui(#[from] UiError),

    #[error(transparent)]
    CliState(#[from] CliStateError),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Http(#[from] HttpError),

    #[error(transparent)]
    Fmt(#[from] fmt::Error),
}

impl ApiError {
    #[track_caller]
    pub fn message<T: fmt::Display>(m: T) -> ApiError {
        crate::error::ApiError::from(ockam_core::Error::new(
            Origin::Application,
            Kind::Unknown,
            m.to_string(),
        ))
    }

    #[track_caller]
    pub fn core<T: fmt::Display>(m: T) -> ockam_core::Error {
        ockam_core::Error::new(Origin::Application, Kind::Unknown, m.to_string())
    }
}

impl From<miette::Error> for ApiError {
    fn from(e: miette::Error) -> Self {
        ApiError::General(e.to_string())
    }
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

    #[error(transparent)]
    Time(#[from] time::error::Parse),

    #[error(transparent)]
    MultiAddr(#[from] ockam_multiaddr::Error),
}

impl From<ParseError> for ockam_core::Error {
    fn from(e: ParseError) -> Self {
        ockam_core::Error::new(Origin::Application, Kind::Parse, e.to_string())
    }
}

#[derive(Debug, thiserror::Error, Diagnostic)]
pub enum UiError {
    #[error(transparent)]
    Dialoguer(#[from] dialoguer::Error),
}

#[derive(Debug, thiserror::Error, Diagnostic)]
pub enum HttpError {
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),

    #[error(transparent)]
    Hyper(#[from] hyper::http::Error),
}
