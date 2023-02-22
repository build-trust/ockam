use std::fmt::{Debug, Display, Formatter};

use crate::version::Version;
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
        writeln!(f, "{}", Version::short())?;
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

macro_rules! gen_from_impl {
    ($t:ty, $c:ident) => {
        impl From<$t> for Error {
            fn from(e: $t) -> Self {
                Error::new(exitcode::$c, e.into())
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
gen_from_impl!(minicbor::encode::Error<std::convert::Infallible>, DATAERR);
gen_from_impl!(minicbor::decode::Error, DATAERR);
gen_from_impl!(ockam::Error, SOFTWARE);
gen_from_impl!(ockam_api::cli_state::CliStateError, SOFTWARE);
gen_from_impl!(ockam_multiaddr::Error, SOFTWARE);
