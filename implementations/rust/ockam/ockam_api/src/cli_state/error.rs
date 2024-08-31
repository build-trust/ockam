#![allow(clippy::unconditional_recursion)]
use miette::Diagnostic;
use ockam_core::Error;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, CliStateError>;

#[derive(Debug, Error, Diagnostic)]
pub enum CliStateError {
    #[error(transparent)]
    #[diagnostic(code("OCK500"))]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    #[diagnostic(code("OCK500"))]
    Serde(#[from] serde_json::Error),

    #[error(transparent)]
    #[diagnostic(code("OCK500"))]
    Fmt(#[from] std::fmt::Error),

    #[error(transparent)]
    #[diagnostic(code("OCK500"))]
    Ockam(#[from] ockam_core::Error),

    #[error("A {resource} named {name} already exists")]
    #[diagnostic(
        code("OCK409"),
        help("Please try using a different name or delete the existing {resource}")
    )]
    AlreadyExists { resource: String, name: String },

    #[error("Unable to find {resource} named {name}")]
    #[diagnostic(code("OCK404"))]
    ResourceNotFound { resource: String, name: String },

    #[error("The path {0} is invalid")]
    #[diagnostic(code("OCK500"))]
    InvalidPath(String),

    #[error("The path is empty")]
    #[diagnostic(code("OCK500"))]
    EmptyPath,

    #[error("{0}")]
    #[diagnostic(code("OCK500"))]
    InvalidData(String),

    #[error("{0}")]
    #[diagnostic(code("OCK500"))]
    InvalidOperation(String),

    #[error("Invalid configuration version '{0}'")]
    #[diagnostic(
        code("OCK500"),
        help("Please try running 'ockam reset' to reset your local configuration")
    )]
    InvalidVersion(String),

    #[diagnostic(code("OCK500"))]
    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

impl From<CliStateError> for ockam_core::Error {
    #[track_caller]
    fn from(e: CliStateError) -> Self {
        match e {
            CliStateError::Ockam(e) => e,
            _ => Error::new(
                ockam_core::errcode::Origin::Application,
                ockam_core::errcode::Kind::Internal,
                e,
            ),
        }
    }
}

impl From<ockam_multiaddr::Error> for CliStateError {
    #[track_caller]
    fn from(e: ockam_multiaddr::Error) -> Self {
        e.into()
    }
}

macro_rules! gen_from_impl {
    ($t:ty) => {
        impl From<$t> for CliStateError {
            #[track_caller]
            fn from(e: $t) -> Self {
                CliStateError::Other(e.into())
            }
        }
    };
}

gen_from_impl!(miette::Error);
gen_from_impl!(&str);
gen_from_impl!(dialoguer::Error);
