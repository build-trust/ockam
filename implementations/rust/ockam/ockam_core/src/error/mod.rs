//! Error and Result types
#![allow(missing_docs, dead_code)] // FIXME DONOTLAND
use crate::compat::{boxed::Box, error::Error as ErrorTrait};
use serde::{Deserialize, Serialize};

use self::code::ErrorCode;

mod code;
mod inner;

/// A module to export the error code in a meaningful way
pub mod errcode {
    pub use super::code::*;
}

// We box the internal error type if an allocator is available — this is (often
// significantly) more efficient in the success path.
#[cfg(feature = "alloc")]
type ErrorData = Box<inner::ErrorData>;
// When an allocator is not available, we represent the internal error inline.
// It should be smaller in this configuration, which avoids much of the cost.
#[cfg(not(feature = "alloc"))]
type ErrorData = Inner;

pub type Result<T, E = Error> = core::result::Result<T, E>;

/// The type of errors returned by Ockam functions.
///
/// Errors store:
///
/// - A set of [error codes](`codes::Code`), which abstractly describe the
///   problem and allow easily matching against specific categories of error.
/// - An open-ended payload, to which arbitrary data can be attached.
/// - The "cause", of this error, if it has not been lost to serialization.
/// - Various debugging information, such as a backtrace and spantrace (which is
///   lost over serialization).
#[derive(Serialize, Deserialize)]
pub struct Error(ErrorData);
impl Error {
    /// Construct a new error given ErrorCodes and a cause.
    #[cold]
    #[track_caller]
    #[cfg(feature = "std")]
    pub fn new<E>(origin: code::Origin, kind: code::Kind, cause: E) -> Self
    where
        E: Into<Box<dyn std::error::Error + Send + Sync>>,
    {
        Self(inner::ErrorData::new(ErrorCode::new(origin, kind), cause).into())
    }

    // FIXME: figure out a better solution here...
    #[cold]
    #[track_caller]
    #[cfg(not(feature = "std"))]
    pub fn new<E>(origin: code::Origin, kind: code::Kind, cause: E) -> Self
    where
        E: core::fmt::Display,
    {
        Self(inner::ErrorData::new(ErrorCode::new(origin, kind), cause).into())
    }

    /// Construct a new error with "unknown" error codes.
    ///
    /// This ideally should not be used inside Ockam.
    #[cold]
    #[cfg(feature = "std")]
    #[track_caller]
    pub fn new_unknown<E>(origin: code::Origin, cause: E) -> Self
    where
        E: Into<Box<dyn crate::compat::error::Error + Send + Sync>>,
    {
        Self::new(origin, code::Kind::Unknown, cause)
    }

    /// Construct a new error without an apparent cause
    ///
    /// This constructor should be used for any error occurring
    /// because of a None unwrap.
    #[cold]
    #[track_caller]
    pub fn new_without_cause(origin: code::Origin, kind: code::Kind) -> Self {
        Self(inner::ErrorData::new_without_cause(origin, kind).into())
    }

    /// Return the [codes](`codes::ErrorCodes`) that identify this error.
    pub fn code(&self) -> ErrorCode {
        self.0.code
    }

    /// Attach additional unstructured information to the error.
    #[must_use]
    pub fn context(mut self, key: &str, val: impl core::fmt::Display) -> Self {
        self.0.add_context(key, &val);
        self
    }
}

impl core::fmt::Debug for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.0.fmt(f)
    }
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.0.fmt(f)
    }
}

impl ErrorTrait for Error {
    #[cfg(feature = "std")]
    fn source(&self) -> Option<&(dyn ErrorTrait + 'static)> {
        if let Some(e) = self.0.cause() {
            let force_coersion: &(dyn ErrorTrait + 'static) = e;
            Some(force_coersion)
        } else {
            None
        }
    }
}
