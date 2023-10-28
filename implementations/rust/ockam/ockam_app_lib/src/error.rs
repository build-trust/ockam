use miette::Diagnostic;
use ockam::compat::tokio::task::JoinError;
use thiserror::Error;

pub type Result<T> = miette::Result<T, Error>;

#[derive(Error, Diagnostic, Debug)]
pub enum Error {
    #[error(transparent)]
    Internal(Box<dyn std::error::Error + Send + Sync + 'static>),

    #[error(transparent)]
    Parse(Box<dyn std::error::Error + Send + Sync + 'static>),

    #[error("{0}")]
    App(String),

    #[error(transparent)]
    Ockam(#[from] ockam_core::Error),

    #[error(transparent)]
    Api(#[from] ockam_api::error::ApiError),

    #[error(transparent)]
    CliState(#[from] ockam_api::cli_state::CliStateError),
}

impl From<JoinError> for Error {
    fn from(e: JoinError) -> Self {
        Error::Internal(e.into())
    }
}

impl From<miette::Report> for Error {
    fn from(e: miette::Report) -> Self {
        Error::App(e.to_string())
    }
}

impl From<&str> for Error {
    fn from(e: &str) -> Self {
        Error::App(e.to_string())
    }
}

impl From<String> for Error {
    fn from(e: String) -> Self {
        Error::App(e)
    }
}

macro_rules! gen_to_parse_err_impl {
    ($t:ty) => {
        impl From<$t> for Error {
            fn from(e: $t) -> Self {
                Error::Parse(e.into())
            }
        }
    };
}

gen_to_parse_err_impl!(serde_json::Error);
gen_to_parse_err_impl!(std::net::AddrParseError);
gen_to_parse_err_impl!(std::string::FromUtf8Error);

macro_rules! gen_to_internal_err_impl {
    ($t:ty) => {
        impl From<$t> for Error {
            fn from(e: $t) -> Self {
                Error::Internal(e.into())
            }
        }
    };
}

gen_to_internal_err_impl!(std::io::Error);
