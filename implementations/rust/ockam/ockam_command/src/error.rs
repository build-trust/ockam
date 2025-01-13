use crate::environment::compile_time_vars::SUPPORT_EMAIL;
use crate::util::exitcode::{self, ExitCode};
use crate::version::Version;
use colorful::Colorful;
use miette::{miette, GraphicalReportHandler};
use miette::{Diagnostic, Report};
use ockam_api::fmt_log;
use ockam_api::terminal::fmt;
use std::fmt::{Debug, Formatter};

pub type Result<T> = miette::Result<T>;

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
    #[error("{0}")]
    Retry(Report),
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
            Error::Retry { .. } => exitcode::SOFTWARE,
        }
    }
}

pub struct ErrorReportHandler;

impl ErrorReportHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ErrorReportHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl miette::ReportHandler for ErrorReportHandler {
    fn debug(&self, error: &dyn Diagnostic, f: &mut Formatter<'_>) -> core::fmt::Result {
        let graphical_report_handler = GraphicalReportHandler::new()
            .with_cause_chain()
            .with_urls(false);

        if f.alternate() {
            return graphical_report_handler.render_report(f, error);
        }

        // Render the graphical report handler output to a string that we can mutate
        let mut graphical_handler_output = String::new();
        let mut error_code = None;
        graphical_report_handler.render_report(&mut graphical_handler_output, error)?;
        // Remove the error code as rendered by the graphical handler and format it as we want
        if let Some(code) = error.code() {
            graphical_handler_output = graphical_handler_output
                .lines()
                .skip(2)
                .collect::<Vec<_>>()
                .join("\n");
            error_code = Some(format!("Error code: {code}"));
        }
        // Add the padding we use in console output and print it
        for line in graphical_handler_output.lines() {
            writeln!(f, "{}{}", fmt::MIETTE_PADDING, line)?;
        }
        writeln!(f)?;

        if let Some(code) = error_code {
            writeln!(f, "{}", fmt_log!("{}", code.dark_gray()))?;
        }
        let version = Version::new().no_color().multiline().to_string();
        for line in version.lines() {
            writeln!(f, "{}", fmt_log!("{}", line.dark_gray()))?;
        }
        write!(
            f,
            "\n{}\n{}",
            fmt_log!(
                "{} {}",
                "If you need help, please email us on".dark_gray(),
                SUPPORT_EMAIL.dark_gray()
            ),
            fmt_log!(
                "{}",
                "We will promptly help you get unstuck and quickly resolve any problems"
                    .dark_gray()
            )
        )?;

        Ok(())
    }
}
