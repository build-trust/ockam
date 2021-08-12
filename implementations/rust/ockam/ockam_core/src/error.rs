//! Error and Result types

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
/// 1. __Error Code__: A `u32` representing the the presise error.
/// 2. __Error Domain__: An error domain string.
///
/// # no_std
/// When the `"std"` feature is not enabled we assume that the Rust Standard
/// Library is not available, the `Error` stores:
///
/// 1. __Error Code__: A `u32` representing the the presise error.
///
#[derive(Serialize, Deserialize, Debug)]
pub struct Error {
    code: u32,

    #[cfg(any(feature = "std", feature = "alloc"))]
    domain: String,
}

/// The type returned by Ockam functions.
pub type Result<T> = core::result::Result<T, Error>;

/// Produces Ok(false), which reads confusingly in auth code.
pub fn deny() -> Result<bool> {
    Ok(false)
}

/// Produces Ok(true), which reads confusingly in auth code.
pub fn allow() -> Result<bool> {
    Ok(true)
}

impl Error {
    /// Creates a new [`Error`].
    #[cfg(all(feature = "no_std", not(feature = "alloc")))]
    pub fn new(code: u32) -> Self {
        Self { code }
    }

    /// Creates a new [`Error`].
    #[cfg(any(feature = "std", feature = "alloc"))]
    pub fn new<S: Into<String>>(code: u32, domain: S) -> Self {
        Self {
            code,
            domain: domain.into(),
        }
    }

    /// Returns an error's domain.
    #[cfg(any(feature = "std", feature = "alloc"))]
    #[inline]
    pub fn domain(&self) -> &String {
        &self.domain
    }

    /// Returns an error's code.
    #[inline]
    pub fn code(&self) -> u32 {
        self.code
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        #[cfg(any(feature = "std", feature = "alloc"))]
        {
            write!(
                f,
                "Error {{ code: {}, domain: \"{}\" }}",
                self.code, self.domain
            )
        }
        #[cfg(all(feature = "no_std", not(feature = "alloc")))]
        {
            write!(f, "Error {{ code: {} }}", self.code)
        }
    }
}

impl crate::compat::error::Error for Error {}

#[cfg(any(feature = "std", feature = "alloc"))]
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

#[cfg(all(feature = "no_std", not(feature = "alloc")))]
#[cfg(test)]
mod no_std_test {
    // These following tests are run for no_std targets without
    // support for heap allocation:
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
