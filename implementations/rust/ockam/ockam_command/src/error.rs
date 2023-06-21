use crate::{exitcode, fmt_log, ExitCode};

use miette::miette;
use miette::Diagnostic;
use std::fmt::Debug;

pub type Result<T> = miette::Result<T, Error>;

#[derive(Debug, thiserror::Error, Diagnostic)]
pub enum Error {
    // ==== 4xx Errors =====
    // Not Found
    #[diagnostic(
        code(OCK404),
        help("Please check the spelling and try again"),
        url("https://docs.ockam.io/errors/OCK404")
    )]
    #[error("Unable to find {resource} named {resource_name}")]
    NotFound {
        resource: String,
        resource_name: String,
    },

    // Unauthorized
    #[diagnostic(
        code(OCK401),
        help("Be sure you are enrolled to the project and have the correct permissions"),
        url("https://docs.ockam.io/errors/OCK401")
    )]
    #[error("Unauthorized to operate on this project as {identity}")]
    Unauthorized { identity: String },

    // Conflict
    #[diagnostic(
        code(OCK409),
        help("Be sure there are no other {resource}'s as {resource_name}"),
        url("https://docs.ockam.io/errors/OCK409")
    )]
    #[error("Conflict with {resource} named {resource_name}")]
    Conflict {
        resource: String,
        resource_name: String,
    },
    // ==== End 4xx Errors =====

    // ==== 5xx Errors ====
    // InternalError
    #[diagnostic(
        code(OCK500),
        help("Please report this issue, with a copy of your logs, to https://github.com/build-trust/ockam/issues"),
        url("https://docs.ockam.io/errors/OCK500")
    )]
    #[error("{error_message}")]
    InternalError {
        error_message: String,
        exit_code: ExitCode,
    },

    // Unavailable
    #[diagnostic(
        code(OCK503),
        help("Please wait a few minutes and try again or restart {resource:?} {resource_name:?}."),
        url("https://docs.ockam.io/errors/OCK503")
    )]
    #[error("{resource} {resource_name} is unavailable")]
    Unavailable {
        resource: String,
        resource_name: String,
    },
    // ==== End 5xx Errors ====
}

impl Error {
    pub fn new(code: ExitCode, err: miette::ErrReport) -> Self {
        assert_ne!(code, 0, "Error's exit code can't be OK");
        Error::InternalError {
            error_message: err.to_string(),
            exit_code: code,
        }
    }

    pub fn new_internal_error(human_err: &str, inner_err_msg: &str) -> Self {
        let msg = format!("{}\n{}", human_err, fmt_log!("{}", inner_err_msg));
        Self::new(exitcode::SOFTWARE, miette!("{}", msg))
    }

    pub fn code(&self) -> ExitCode {
        match self {
            Error::NotFound { .. } => exitcode::SOFTWARE,
            Error::Unauthorized { .. } => exitcode::NOPERM,
            Error::Conflict { .. } => exitcode::SOFTWARE,
            Error::InternalError { exit_code, .. } => *exit_code,
            Error::Unavailable { .. } => exitcode::UNAVAILABLE,
        }
    }
}

macro_rules! gen_from_impl {
    ($t:ty, $c:ident) => {
        impl From<$t> for Error {
            fn from(e: $t) -> Self {
                use miette::miette;
                Error::new(exitcode::$c, miette!(e.to_string()))
            }
        }
    };
}

gen_from_impl!(std::io::Error, IOERR);
gen_from_impl!(std::fmt::Error, SOFTWARE);
gen_from_impl!(std::net::AddrParseError, DATAERR);
gen_from_impl!(hex::FromHexError, DATAERR);
gen_from_impl!(serde_bare::error::Error, DATAERR);
gen_from_impl!(serde_json::Error, DATAERR);
gen_from_impl!(serde_yaml::Error, DATAERR);
gen_from_impl!(minicbor::encode::Error<std::convert::Infallible>, DATAERR);
gen_from_impl!(minicbor::decode::Error, DATAERR);
gen_from_impl!(ockam::Error, SOFTWARE);
gen_from_impl!(ockam_api::cli_state::CliStateError, SOFTWARE);
gen_from_impl!(ockam_multiaddr::Error, SOFTWARE);
gen_from_impl!(miette::ErrReport, SOFTWARE);
gen_from_impl!(time::error::Parse, DATAERR);
