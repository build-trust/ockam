use super::code::ErrorCode;
use crate::compat::{boxed::Box, error::Error as ErrorTrait, string::String};
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
    // TODO: these should not be disabled for no_std, but: not as simple as
    // `#[cfg()]`ing the fields out, as this changes the serialization format.
    payload: Vec<PayloadEntry>,
    source_loc: Location,
    #[serde(skip, default)]
    cause: Option<BoxDynErr>,
    #[serde(skip, default)]
    local: Option<Vec<LocalPayloadEntry>>,
}

impl ErrorData {
    #[cold]
    #[track_caller]
    pub(super) fn new<E>(code: ErrorCode, cause: E) -> Self
    where
        E: Into<Box<dyn ErrorTrait + Send + Sync>>,
    {
        Self::new_inner(code, cause.into(), core::any::type_name::<E>())
    }

    #[cold]
    #[track_caller]
    pub(super) fn new_inner(
        code: ErrorCode,
        cause: Box<dyn ErrorTrait + Send + Sync>,
        type_name: &'static str,
    ) -> Self {
        let location = core::panic::Location::caller();
        let debug = crate::compat::format!("{:?}", cause);
        let display = crate::compat::format!("{}", cause);
        #[allow(unused_mut)]
        let mut local: Vec<LocalPayloadEntry> = vec![];

        #[cfg(all(feature = "std", feature = "error-traces"))]
        if trace_config::BACKTRACE_ENABLED.get() {
            local.push(LocalPayloadEntry::Backtrace(backtrace::Backtrace::new()));
        }
        #[cfg(all(feature = "std", feature = "error-traces"))]
        if trace_config::SPANTRACE_ENABLED.get() {
            local.push(LocalPayloadEntry::Spantrace(
                tracing_error::SpanTrace::capture(),
            ));
        }
        Self {
            code,
            cause: Some(cause),
            source_loc: location.into(),
            local: Some(local),
            payload: vec![PayloadEntry::Cause {
                display,
                debug,
                type_name: type_name.to_owned(),
            }],
        }
    }
    #[cold]
    pub fn add_context(&mut self, key: &str, val: &dyn core::fmt::Display) {
        self.payload.push(PayloadEntry::Info(
            key.into(),
            crate::compat::format!("{}", val),
        ));
    }

    pub fn cause(&self) -> Option<&(dyn ErrorTrait + Send + Sync + 'static)> {
        self.cause.as_deref()
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
            line: p.line().into(),
            column: p.column().into(),
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
