//! Error and Result types

use crate::lib::{fmt::Formatter, Display, String};
use serde::{Deserialize, Serialize};

/// The type of errors returned by Ockam functions.
///
/// This type has two implementations that are switched depending on
/// whether the `"std"` Cargo feature is enabled.
///
/// # std
/// When the `"std"` feature is enabled and the Rust Standard Library is
/// available, the `Error` stores:
///
/// 1. __Error Code__: A `u32` representing the the presise error.
/// 2. __Error Domain__: An error domain string.
///
/// # no_std
/// When the `"std"` feature is not enabled we assume that the Rust Standard
/// Library is not available, the `Error` stores:
///
/// 1. __Error Code__: A `u32` representing the the presise error.
///
#[derive(Debug, Serialize, Deserialize)]
pub struct Error {
    code: u32,

    #[cfg(feature = "std")]
    domain: String,
}

/// The type returned by Ockam functions.
pub type Result<T> = crate::lib::Result<T, Error>;

impl Error {
    /// Creates a new [`Error`].
    #[cfg(not(feature = "std"))]
    pub fn new(code: u32) -> Self {
        Self { code }
    }

    /// Creates a new [`Error`].
    #[cfg(feature = "std")]
    pub fn new(code: u32, domain: &'static str) -> Self {
        Self {
            code,
            domain: domain.into(),
        }
    }

    /// Returns an error's domain.
    #[cfg(feature = "std")]
    #[inline]
    pub fn domain(&self) -> &str {
        self.domain.as_str()
    }

    /// Returns an error's code.
    #[inline]
    pub fn code(&self) -> u32 {
        self.code
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> crate::lib::fmt::Result {
        #[cfg(feature = "std")]
        {
            write!(
                f,
                "Error {{ code: {}, domain: \"{}\" }}",
                self.code, self.domain
            )
        }
        #[cfg(not(feature = "std"))]
        {
            write!(f, "Error {{ code: {} }}", self.code)
        }
    }
}

#[cfg(feature = "std")]
impl crate::lib::error::Error for Error {}

#[cfg(feature = "std")]
#[cfg(test)]
mod std_test {
    use super::*;

    #[test]
    fn can_be_created() {
        let _error = Error::new(1000, "SOME_ERROR_DOMAIN");
    }

    #[test]
    fn code_returns_provided_code() {
        let error = Error::new(1000, "SOME_ERROR_DOMAIN");
        assert_eq!(error.code(), 1000);
        assert_eq!(error.code, 1000);
    }

    #[test]
    fn domain_returns_provided_domain() {
        let error = Error::new(1000, "SOME_ERROR_DOMAIN");
        assert_eq!(error.domain(), "SOME_ERROR_DOMAIN");
        assert_eq!(error.domain, "SOME_ERROR_DOMAIN");
    }

    #[test]
    fn can_be_displayed() {
        let error = Error::new(1000, "SOME_ERROR_DOMAIN");
        assert_eq!(
            format!("{}", error),
            "Error { code: 1000, domain: \"SOME_ERROR_DOMAIN\" }"
        );
    }

    #[test]
    fn can_be_debugged() {
        let error = Error::new(1000, "SOME_ERROR_DOMAIN");
        assert_eq!(
            format!("{:?}", error),
            "Error { code: 1000, domain: \"SOME_ERROR_DOMAIN\" }"
        );
    }
}

#[cfg(not(feature = "std"))]
#[cfg(test)]
mod no_std_test {
    // These following tests are only run when the std feature in not enabled
    // cargo test --no-default-features

    use super::*;

    #[test]
    fn can_be_created_and_code_returns_provided_code() {
        let error = Error::new(1000);
        assert_eq!(error.code(), 1000);
        assert_eq!(error.code, 1000);
    }
}
