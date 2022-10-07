use std::fmt::{Debug, Display, Formatter};

use crate::util::ConfigError;
use crate::{exitcode, ExitCode};

pub type Result<T> = std::result::Result<T, Error>;

pub struct Error {
    code: ExitCode,
    inner: anyhow::Error,
}

impl Error {
    pub fn new(code: ExitCode, err: anyhow::Error) -> Self {
        assert_ne!(code, 0, "Error's exit code can't be OK");
        Self { code, inner: err }
    }

    pub fn code(&self) -> ExitCode {
        self.code
    }
}

impl Debug for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.inner, f)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{code: {}, err: {}}}", self.code, self.inner)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.inner.as_ref())
    }
}

impl From<ConfigError> for Error {
    fn from(e: ConfigError) -> Self {
        Error::new(exitcode::CONFIG, e.into())
    }
}

impl From<ockam::Error> for Error {
    fn from(e: ockam::Error) -> Self {
        Error::new(exitcode::SOFTWARE, e.into())
    }
}

impl From<anyhow::Error> for Error {
    fn from(e: anyhow::Error) -> Self {
        Error::new(exitcode::SOFTWARE, e)
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
