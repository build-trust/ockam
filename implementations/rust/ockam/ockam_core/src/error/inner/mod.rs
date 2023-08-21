#[allow(unused)]
use super::code::{Kind, Origin};
#[cfg(feature = "std")]
use crate::compat::{borrow::ToOwned, vec::Vec};
use crate::compat::{boxed::Box, error::Error as ErrorTrait, string::String};
use crate::error::code::ErrorCode;
use serde::{Deserialize, Serialize};

mod formatting;
#[cfg(all(feature = "std", feature = "error-traces"))]
mod trace_config;

type BoxDynErr = Box<dyn ErrorTrait + Send + Sync + 'static>;

/// The internal error type, which [`super::NewError`] wraps.
///
/// When an allocator is available, this is `Box`ed for performance reasons,
/// (although some of the fields here are a little goofy in low level contexts).
#[derive(Serialize, Deserialize)]
pub(super) struct ErrorData {
    pub(super) code: ErrorCode,
    // FIXME: `#[cfg()]`ing the fields out changes the serialization format. Not good.
    #[cfg(feature = "std")]
    payload: Vec<PayloadEntry>,
    #[cfg(feature = "std")]
    pub(super) source_loc: Location,
    #[cfg(feature = "std")]
    #[serde(skip, default)]
    cause: Option<BoxDynErr>,
    #[cfg(feature = "std")]
    #[serde(skip, default)]
    local: Option<Vec<LocalPayloadEntry>>,
}

impl ErrorData {
    #[cold]
    #[track_caller]
    #[cfg(feature = "std")]
    pub(super) fn new<E>(code: ErrorCode, cause: E) -> Self
    where
        E: Into<Box<dyn ErrorTrait + Send + Sync>>,
    {
        Self::new_inner(code, Some(cause.into()), Some(core::any::type_name::<E>()))
    }
    #[cold]
    #[track_caller]
    #[cfg(feature = "std")]
    pub(super) fn new_without_cause(origin: Origin, kind: Kind) -> Self {
        Self::new_inner(ErrorCode::new(origin, kind), None, None)
    }

    // FIXME
    #[cold]
    #[track_caller]
    #[cfg(not(feature = "std"))]
    pub(super) fn new<E>(code: ErrorCode, _cause: E) -> Self {
        Self { code }
    }

    #[cold]
    #[track_caller]
    #[cfg(not(feature = "std"))]
    pub(super) fn new_without_cause(origin: Origin, kind: Kind) -> Self {
        Self {
            code: ErrorCode::new(origin, kind),
        }
        // Self::new_inner(ErrorCode::new(origin, kind), None, None)
    }

    #[cold]
    #[track_caller]
    #[cfg(feature = "std")]
    pub(super) fn new_inner(
        code: ErrorCode,
        cause: Option<Box<dyn ErrorTrait + Send + Sync>>,
        type_name: Option<&'static str>,
    ) -> Self {
        let location = core::panic::Location::caller();
        #[allow(unused_mut)]
        let mut local: Vec<LocalPayloadEntry> = vec![];

        #[cfg(all(feature = "std", feature = "backtrace"))]
        if trace_config::BACKTRACE_ENABLED.get() {
            // This is called everytime an Error instance is created and takes up to 300ms
            // on a fast CPU, which is way slower than expected from such type of operation.
            // Therefore, this feature is disabled by default, but can be used.
            // It's also possible to replace `Backtrace::new()` with `Backtrace::new_unresolved()`
            // which is much faster.
            local.push(LocalPayloadEntry::Backtrace(backtrace::Backtrace::new()));
        }

        #[cfg(all(feature = "std", feature = "tracing-error"))]
        if trace_config::SPANTRACE_ENABLED.get() {
            local.push(LocalPayloadEntry::Spantrace(
                tracing_error::SpanTrace::capture(),
            ));
        }

        let mut payload = vec![];
        if let Some(cause) = cause.as_ref() {
            let debug = crate::compat::format!("{:?}", cause);
            let display = crate::compat::format!("{}", cause);
            payload.push(PayloadEntry::Cause {
                display,
                debug,
                type_name: type_name.unwrap_or("<unknown>").to_owned(),
            });
        }

        Self {
            code,
            cause,
            source_loc: location.into(),
            local: Some(local),
            payload,
        }
    }

    #[cold]
    pub fn add_context(&mut self, _key: &str, _val: &dyn core::fmt::Display) {
        #[cfg(feature = "std")]
        self.payload.push(PayloadEntry::Info(
            _key.into(),
            crate::compat::format!("{}", _val),
        ));
    }

    #[cfg(feature = "std")]
    pub fn cause(&self) -> Option<&(dyn ErrorTrait + Send + Sync + 'static)> {
        self.cause.as_deref()
    }

    #[cfg(not(feature = "std"))]
    pub fn cause(&self) -> Option<&(dyn ErrorTrait + Send + Sync + 'static)> {
        None
    }
}

#[derive(Serialize, Deserialize, Debug)]
enum PayloadEntry {
    // `Display` for the error cause (note that this may instead be used for the
    // `message`).
    Cause {
        display: String,
        debug: String,
        type_name: String,
    },
    // Miscellaneous info — type name of the worker that caused the error for
    // example, or a human description of the cause.
    Info(String, String),
}

// Information that is only relevant on this machine. May be logged, so should
// not contain sensitive data, but is discarded when serializing.
#[derive(Debug)]
enum LocalPayloadEntry {
    #[cfg(all(feature = "std", feature = "backtrace"))]
    Backtrace(backtrace::Backtrace),
    #[cfg(all(feature = "std", feature = "tracing-error"))]
    Spantrace(tracing_error::SpanTrace),
}

/// Serializable version of `core::panic::Location`, as returned by
/// `core::panic::Location::caller()`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub(crate) struct Location {
    file: String,
    line: u32,
    column: u32,
}

impl From<&core::panic::Location<'_>> for Location {
    fn from(p: &core::panic::Location<'_>) -> Self {
        Self {
            file: p.file().into(),
            line: p.line(),
            column: p.column(),
        }
    }
}

impl core::fmt::Display for Location {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // This format is compatible with most editors, and is used by rust (and
        // many others) for source location output.
        write!(f, "{}:{}:{}", self.file, self.line, self.column)
    }
}
