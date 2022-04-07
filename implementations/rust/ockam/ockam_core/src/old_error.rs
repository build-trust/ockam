//! Legacy Error and Result types.
//!
//! Over time we will be migrating away from this.

use crate::compat::string::String;
use core::fmt::{self, Display, Formatter};
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
/// 1. __Error Code__: A `u32` representing the precise error.
/// 2. __Error Domain__: An error domain string.
///
/// # no_std
/// When the `"std"` feature is not enabled we assume that the Rust Standard
/// Library is not available, the `Error` stores:
///
/// 1. __Error Code__: A `u32` representing the precise error.
///
#[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
pub struct Error {
    code: u32,

    #[cfg(feature = "alloc")]
    domain: String,
}

/// The result type returned by Ockam functions.
pub type Result<T, E = Error> = core::result::Result<T, E>;

impl Error {
    /// Creates a new [`Error`].
    #[cfg(not(feature = "alloc"))]
    pub fn new(code: u32) -> Self {
        Self { code }
    }

    /// Creates a new [`Error`].
    #[cfg(feature = "alloc")]
    pub fn new<S: Into<String>>(code: u32, domain: S) -> Self {
        Self {
            code,
            domain: domain.into(),
        }
    }

    /// Return an error's domain.
    #[inline]
    #[cfg(feature = "alloc")]
    pub fn domain(&self) -> &str {
        &self.domain
    }

    /// Return an error's code.
    #[inline]
    pub fn code(&self) -> u32 {
        self.code
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        #[cfg(feature = "alloc")]
        {
            write!(
                f,
                "Error {{ code: {}, domain: \"{}\" }}",
                self.code, self.domain
            )
        }
        #[cfg(not(feature = "alloc"))]
        {
            write!(f, "Error {{ code: {} }}", self.code)
        }
    }
}

impl crate::compat::error::Error for Error {}

impl From<core::convert::Infallible> for Error {
    fn from(e: core::convert::Infallible) -> Self {
        match e {} // Infallible is uninhabited
    }
}

#[cfg(feature = "alloc")]
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

#[cfg(not(feature = "alloc"))]
#[cfg(test)]
mod no_std_test {
    // The following tests are run for no_std targets without support
    // for heap allocation:
    //
    //     cargo test --no-default-features --features="no_std"
    use super::*;

    #[test]
    fn can_be_created_and_code_returns_provided_code() {
        let error = Error::new(1000);
        assert_eq!(error.code(), 1000);
        assert_eq!(error.code, 1000);
    }
}
