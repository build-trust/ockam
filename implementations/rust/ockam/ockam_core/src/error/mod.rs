//! Error and Result types
#![allow(missing_docs, dead_code)] // FIXME DONOTLAND
use crate::compat::{boxed::Box, error::Error as ErrorTrait};
use serde::{Deserialize, Serialize};

pub mod code;
mod inner;
pub mod none;

// We box the internal error type if an allocator is available — this is (often
// significantly) more efficient in the success path.
#[cfg(feature = "alloc")]
type ErrorData = Box<inner::ErrorData>;
// When an allocator is not available, we represent the internal error inline.
// It should be smaller in this configuration, which avoids much of the cost.
#[cfg(not(feature = "alloc"))]
type ErrorData = Inner;

pub type Result<T> = std::result::Result<T, Error2>;

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
pub struct Error2(ErrorData);
impl Error2 {
    /// Construct a new error given ErrorCodes and a cause.
    #[cold]
    #[track_caller]
    pub fn new<E>(code: code::ErrorCode, cause: E) -> Self
    where
        E: Into<Box<dyn crate::compat::error::Error + Send + Sync>>,
    {
        Self(inner::ErrorData::new(code, cause).into())
    }

    /// Construct a new error with "unknown" error codes.
    ///
    /// This ideally should not be used inside Ockam.
    #[cold]
    pub fn new_unknown<E>(cause: E) -> Self
    where
        E: Into<Box<dyn crate::compat::error::Error + Send + Sync>>,
    {
        Self::new(code::ErrorCode::unknown(), cause)
    }

    /// Construct a new error without an apparent cause
    ///
    /// This constructor should be used for any error occurring
    /// because of a None unwrap.
    #[cold]
    pub fn new_without_cause(code: code::ErrorCode) -> Self {
        Self(inner::ErrorData::new_without_cause(code).into())
    }

    /// Return the [codes](`codes::ErrorCodes`) that identify this error.
    pub fn code(&self) -> code::ErrorCode {
        self.0.code
    }

    /// Attach additional unstructured informaton to the error.
    #[must_use]
    pub fn context(mut self, key: &str, val: impl core::fmt::Display) -> Self {
        self.0.add_context(key, &val);
        self
    }
}

impl From<crate::old_error::Error> for Error2 {
    #[cold]
    #[track_caller]
    fn from(src: crate::old_error::Error) -> Self {
        let origin = match src.domain() {
            o if o.starts_with("OCKAM_NODE") => code::Origin::Node,
            o if o.starts_with("OCKAM_EXECUTOR") => code::Origin::Executor,
            o if o.starts_with("OCKAM_TRANSPORT") => code::Origin::Transport,
            o if o.starts_with("OCKAM_KEX") => code::Origin::KeyExchange,
            o if o.starts_with("OCKAM_ENTITY")
                || o.starts_with("OCKAM_IDENTITY")
                || o.starts_with("OCKAM_CREDENTIAL") =>
            {
                code::Origin::Identity
            }
            o if o.starts_with("OCKAM_VAULT") => code::Origin::Vault,
            o if o.starts_with("OCKAM_FFI") => code::Origin::Api,
            o if o.starts_with("OCKAM") => code::Origin::Other,
            _ => code::Origin::Unknown,
        };
        let kind = code::Kind::Unknown;
        let ec = code::ErrorCode::new_with_extra(origin, kind, src.code() as i32);
        let orig_domain = src.domain().clone();
        Error2::new(ec, src).context("domain", orig_domain)
    }
}

impl core::fmt::Debug for Error2 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.0.fmt(f)
    }
}

impl core::fmt::Display for Error2 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(feature = "std")]
impl ErrorTrait for Error2 {
    fn source(&self) -> Option<&(dyn ErrorTrait + 'static)> {
        if let Some(e) = self.0.cause() {
            let force_coersion: &(dyn ErrorTrait + 'static) = e;
            Some(force_coersion)
        } else {
            None
        }
    }
}
