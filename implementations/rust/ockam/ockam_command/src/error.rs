use crate::util::exitcode::{self, ExitCode};
use crate::version::Version;
use crate::{fmt_heading, fmt_log};
use colorful::Colorful;
use miette::miette;
use miette::Diagnostic;
use std::fmt::{Debug, Formatter};

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

    #[diagnostic(
        code(OCK401),
        help("Make sure you are enrolled before running this command"),
        url("https://docs.ockam.io/errors/OCK401")
    )]
    #[error("There is no default project defined. Please enroll or create a project.")]
    NotEnrolled,

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
    #[track_caller]
    pub fn new(code: ExitCode, err: miette::ErrReport) -> Self {
        assert_ne!(code, 0, "Error's exit code can't be OK");
        Error::InternalError {
            error_message: err.to_string(),
            exit_code: code,
        }
    }

    #[track_caller]
    pub fn arg_validation<T: Debug>(arg: &str, value: T, err: Option<&str>) -> Self {
        let err = err.map(|e| format!(": {e}")).unwrap_or_default();
        let msg = format!("invalid value '({value:?})' for '{arg}' {err}");
        Self::new(exitcode::USAGE, miette!(msg))
    }

    #[track_caller]
    pub fn new_internal_error(msg: &str) -> Self {
        Self::new(exitcode::SOFTWARE, miette!(msg.to_string()))
    }

    pub fn code(&self) -> ExitCode {
        match self {
            Error::NotFound { .. } => exitcode::SOFTWARE,
            Error::Unauthorized { .. } => exitcode::NOPERM,
            Error::NotEnrolled => exitcode::NOPERM,
            Error::Conflict { .. } => exitcode::SOFTWARE,
            Error::InternalError { exit_code, .. } => *exit_code,
            Error::Unavailable { .. } => exitcode::UNAVAILABLE,
        }
    }
}

pub struct ErrorReportHandler;

impl ErrorReportHandler {
    pub fn new() -> Self {
        Self
    }

    #[allow(dead_code)]
    // The cause of a [`Diagnostic`] could be both a [`Diagnostic`] or a [`std::error::Error`].
    // The cause of a [`std::error::Error`] can only be another [`std::error::Error`].
    fn print_causes(f: &mut Formatter, root: &dyn Diagnostic) -> core::fmt::Result {
        let mut error_source = None;
        let mut diagnostic_source = Some(root);
        loop {
            if let Some(source) = diagnostic_source {
                diagnostic_source = source.diagnostic_source();
                if diagnostic_source.is_none() {
                    error_source = source.source();
                } else {
                    error_source = None;
                }
            } else if let Some(source) = error_source {
                error_source = source.source();
            }

            if let Some(source) = error_source {
                writeln!(f, "       > {}", source)?;
            } else if let Some(source) = diagnostic_source {
                writeln!(f, "       > {}", source)?;
            } else {
                break;
            }
        }
        Ok(())
    }
}

impl Default for ErrorReportHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl miette::ReportHandler for ErrorReportHandler {
    fn debug(&self, error: &dyn Diagnostic, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if f.alternate() {
            return Debug::fmt(error, f);
        }

        writeln!(f, "\n{}\n", fmt_heading!("{}", "Error:".red()))?;

        // Try to extract the source message from the error, and disregard the rest. If
        // possible replace the new lines w/ fmt_log! outputs.
        let error_message = match error.source() {
            Some(err) => format!("{}", err),
            None => format!("{}", error),
        };
        error_message.lines().for_each(|line| {
            let _ = writeln!(f, "{}", fmt_log!("{}", line));
        });

        if let Some(help) = error.help() {
            writeln!(f, "{}", fmt_log!("{}", help))?;
        }

        // TODO: wait until we have the dedicated documentation page for errors
        // if let Some(url) = error.url() {
        //     writeln!(f, "{}", fmt_log!("{}", url))?;
        // }

        // Output the error code and version code.
        let code_as_str = match error.code() {
            Some(code) => code.to_string(),
            None => "OCK500".to_string(),
        };

        // TODO: Display the cause of the error in a nicely formatted way; skip for now.
        // Self::print_causes(f, error)?;

        let code_message = format!("Error code: {}", code_as_str).dark_gray();
        let version_message = format!("version: {}", Version::short()).dark_gray();
        let footer_message = format!("\n{}\n{}", code_message, version_message);
        footer_message.split('\n').for_each(|line| {
            let _ = writeln!(f, "{}", fmt_log!("{}", line));
        });

        writeln!(
            f,
            "\n{}\n{}",
            fmt_log!(
                "{}",
                "If you need help, please create an issue on our github: ".dark_gray()
            ),
            fmt_log!(
                "{}",
                "https://github.com/build-trust/ockam/issues/new/choose".dark_gray()
            )
        )?;

        write!(f, "\n{}", fmt_heading!("{}", ""))?;

        Ok(())
    }
}

macro_rules! gen_from_impl {
    ($t:ty, $c:ident) => {
        impl From<$t> for Error {
            #[track_caller]
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
gen_from_impl!(ockam_api::error::ApiError, SOFTWARE);
gen_from_impl!(ockam_multiaddr::Error, SOFTWARE);
gen_from_impl!(miette::ErrReport, SOFTWARE);
gen_from_impl!(time::error::Parse, DATAERR);
gen_from_impl!(dialoguer::Error, DATAERR);
