use crate::util::ConfigError;
use crate::{exitcode, ExitCode};
use tracing::error;

pub type Result<T> = std::result::Result<T, Error>;

pub struct Error(ExitCode);

impl Error {
    pub fn new(code: ExitCode) -> Self {
        assert!(code > 0, "Exit code can't be OK");
        Self(code)
    }

    pub fn code(&self) -> ExitCode {
        self.0
    }
}

impl From<ConfigError> for Error {
    fn from(e: ConfigError) -> Self {
        error!("{e}");
        Error::new(exitcode::CONFIG)
    }
}

impl From<anyhow::Error> for Error {
    fn from(e: anyhow::Error) -> Self {
        error!("{e}");
        Error::new(exitcode::SOFTWARE)
    }
}
