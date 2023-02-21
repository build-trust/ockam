use std::convert::Infallible;
use std::fmt::{Debug, Display, Formatter};

use crate::{exitcode, ExitCode};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct Error {
    code: ExitCode,
    description: String,
    cause: Option<String>,
}

impl Error {
    pub fn new(code: ExitCode, err: anyhow::Error) -> Self {
        assert_ne!(code, 0, "Error's exit code can't be OK");
        Self {
            code,
            description: err.to_string(),
            cause: err.source().map(|s| s.to_string()),
        }
    }

    pub fn code(&self) -> ExitCode {
        self.code
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(cause) = &self.cause {
            writeln!(f, "{}. Caused by: {}", self.description, cause)?;
        } else {
            writeln!(f, "{}", self.description)?;
        }
        Ok(())
    }
}

impl std::error::Error for Error {}

impl From<anyhow::Error> for Error {
    fn from(e: anyhow::Error) -> Self {
        Error::new(exitcode::SOFTWARE, e)
    }
}

impl From<ockam::Error> for Error {
    fn from(e: ockam::Error) -> Self {
        Error::new(exitcode::SOFTWARE, e.into())
    }
}

impl From<ockam_api::cli_state::CliStateError> for Error {
    fn from(e: ockam_api::cli_state::CliStateError) -> Self {
        Error::new(exitcode::SOFTWARE, e.into())
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::new(exitcode::IOERR, e.into())
    }
}

impl From<ockam_multiaddr::Error> for Error {
    fn from(e: ockam_multiaddr::Error) -> Self {
        Error::new(exitcode::SOFTWARE, e.into())
    }
}

impl From<minicbor::decode::Error> for Error {
    fn from(e: minicbor::decode::Error) -> Self {
        Error::new(exitcode::DATAERR, e.into())
    }
}

impl From<minicbor::encode::Error<Infallible>> for Error {
    fn from(e: minicbor::encode::Error<Infallible>) -> Self {
        Error::new(exitcode::DATAERR, e.into())
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::new(exitcode::DATAERR, e.into())
    }
}

impl From<std::net::AddrParseError> for Error {
    fn from(e: std::net::AddrParseError) -> Self {
        Error::new(exitcode::SOFTWARE, e.into())
    }
}
