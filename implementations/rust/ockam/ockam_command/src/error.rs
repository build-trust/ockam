use crate::version::Version;
use crate::{exitcode, fmt_log, ExitCode};
use colorful::Colorful;

use miette::miette;
use miette::Diagnostic;
use std::fmt::Debug;

pub type Result<T> = std::result::Result<T, Error>;

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
    InteralError {
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
        Error::InteralError {
            error_message: err.to_string(),
            exit_code: code,
        }
    }

    pub fn new_software_error(human_err: &str, inner_err_msg: &str) -> Self {
        let msg = format!("{}\n{}", human_err, fmt_log!("{}", inner_err_msg));
        Self::new(exitcode::SOFTWARE, miette!("{}", msg))
    }

    pub fn code(&self) -> ExitCode {
        match self {
            Error::NotFound { .. } => exitcode::SOFTWARE,
            Error::Unauthorized { .. } => exitcode::NOPERM,
            Error::Conflict { .. } => exitcode::SOFTWARE,
            Error::InteralError { exit_code, .. } => *exit_code,
            Error::Unavailable { .. } => exitcode::UNAVAILABLE,
        }
    }
}

impl From<anyhow::Error> for Error {
    fn from(e: anyhow::Error) -> Self {
        Error::new(exitcode::SOFTWARE, miette!(e.to_string()))
    }
}

pub struct ErrorReportHandler;
impl ErrorReportHandler {
    pub fn new() -> Self {
        Self
    }
}
impl miette::ReportHandler for ErrorReportHandler {
    fn debug(&self, error: &dyn Diagnostic, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if f.alternate() {
            return core::fmt::Debug::fmt(error, f);
        }
        let code_as_str = match error.code() {
            Some(code) => code.to_string(),
            None => "OCK500".to_string(),
        };

        writeln!(
            f,
            "{} {}\n",
            code_as_str
                .color(crate::terminal::OckamColor::FmtERRORBackground.color())
                .bold(),
            error
        )?;

        if let Some(help) = error.help() {
            writeln!(f, "{}", fmt_log!("{}", help))?;
        }

        // TODO: wait until we have the dedicated documentation page for errors
        // if let Some(url) = error.url() {
        //     writeln!(f, "{}", fmt_log!("{}", url))?;
        // }

        writeln!(
            f,
            "{}",
            fmt_log!("{}", Version::short().to_string().light_gray())
        )?;

        Ok(())
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
