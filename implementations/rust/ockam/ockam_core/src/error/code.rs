//! Ockam error code enumerations.
//!
//! These are deliberately abstract, and do not cover all possible errors in
//! detail. Some of the motivation behind this includes:
//!
//! 1. Allow code which wishes to categorize, filter, or otherwise handle errors
//!    to do so generically without forcing them to hard-code numeric values.
//! 2. To avoid each component needing to choose globally unique error numbers.
use serde::{Deserialize, Serialize};

/// A set of abstract error codes describing an error. See the [module-level
/// documentation](crate::error::codes) for details.
///
/// The fields of this struct are `pub` for matching, but you need to go through
/// one of the [constructor functions](ErrorCode::new) to create one of these
/// (and not a literal), as it is a `#[non_exhaustive]` type (which may change in
/// the future, since it's unclear if this provides value).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Copy, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ErrorCode {
    // Maintenance note: Don't reorder these fields or cfg them out, it's
    // somewhat important for serialization (at least via BARE) that these are
    // the same in all configurations -- therefore, all items in this struct
    // should be reasonable to include on embedded as well (going any higher
    // than 64 bits seems likely to be too far in that context -- even this much
    // is perhaps pushing it).
    /// The [`Origin`] of this error.
    pub origin: Origin,
    /// The [`Kind`] of this error.
    pub kind: Kind,
    /// An additional identifying numeric payload, or 0 if none is relevant.
    ///
    /// For example, it would be reasonable for this field to hold:
    /// - HTTP status code
    /// - OS `errno`/`GetLastError` code
    /// - The exit status returned by a subprocess
    /// - A numeric error code from some other system
    /// - Et cetera.
    ///
    /// But should generally not be used to hold non-identifying metadata, such
    /// as the date, device IDs, as that information should be stored on the
    /// payload itself.
    ///
    /// Concretely: two `ErrorCode` with different `extra` values should
    /// identify types of errors.
    // TODO: is 32 bits okay on embedded? This puts us to 64 bits for the
    // structure in practice, but means that we'll alwasy be able to hold OS
    // errors, as well as `old_error::Error::code`.
    pub extra: i32,
}

impl ErrorCode {
    /// Construct the `ErrorCode` for an error.
    #[cold]
    pub fn new(origin: Origin, kind: Kind) -> Self {
        Self {
            origin,
            kind,
            extra: 0,
        }
    }
    /// Construct the `ErrorCode` for an error which contains an additional
    /// numeric payload.
    #[cold]
    pub fn new_with_extra(origin: Origin, kind: Kind, extra: i32) -> Self {
        Self {
            origin,
            kind,
            extra,
        }
    }

    /// Construct an error code with very little useful information
    #[cold]
    pub fn unknown() -> Self {
        Self {
            origin: Origin::Unknown,
            kind: Kind::Unknown,
            extra: 0,
        }
    }

    /// Attach an origin and/or kind to the error, without risk of overwriting more
    /// precise information value.
    #[must_use]
    pub fn update_unknown(
        mut self,
        o: impl Into<Option<Origin>>,
        k: impl Into<Option<Kind>>,
    ) -> Self {
        if let (Origin::Unknown, Some(o)) = (self.origin, o.into()) {
            self.origin = o;
        }
        if let (Kind::Unknown, Some(k)) = (self.kind, k.into()) {
            self.kind = k;
        }
        self
    }
}

/// Origin indicates the abstract source of an error.
///
/// Note that [`Error`](super::Error) should already contain precise origin
/// information (file, line) where the error originated from.
///
// Internal note: Once we stabilise the API, we should not remove these, just stop emitting them.
#[repr(u8)]
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Copy, Serialize, Deserialize)]
pub enum Origin {
    /// An error for which there is no way to determine a more specific origin.
    ///
    /// Eventually this should also be used for errors which, during
    /// deserialization, have an unknown `Origin` (for now this is too
    /// error-prone for various reasons).
    Unknown = 0,
    /// Reserved for errors emitted by applications using ockam.
    Application = 1,
    /// An error originating from the vault.
    Vault = 2,
    /// Errors emitted by the transport layer.
    Transport = 3,
    /// Errors from some part of the node implementation — the router or relay,
    /// for example.
    Node = 4,
    /// Errors from the surface API — for example: the FFI layer.
    Api = 5,
    /// Errors from within the identity-management code.
    Identity = 6,
    /// Errors from the secure channel implementation.
    Channel = 7,
    /// Errors occurring from the one of the key exchange implementations.
    KeyExchange = 8,
    /// An error which occurs in the executor (e.g. `ockam_executor`, since
    /// `tokio` errors will likely come from elsewhere).
    Executor = 9,
    /// Other errors from within `ockam` or `ockam_core`.
    Core = 10,
    /// Ockam protocol crate
    Ockam = 11,
    /// Errors from within the Ockam authorization code.
    Authorization = 12,
    /// Errors from other sources, such as libraries extending `ockam`.
    ///
    /// Note: The actual source (file, line, ...) will (hopefully) be available
    /// on the error itself, as one of the members of the payload.
    Other = 13,
    // This is a `#[non_exhaustive]` enum — we're free to add more variants
    // here. Do not add any which contain payloads (it should stay a "C style
    // enum"). Payload information should be added to the error itself.
}

/// Category indicates "what went wrong", in abstract terms.
///
/// # Choosing a `Kind`
///
/// - [`Kind::Io`], [`Kind::Protocol`], and [`Kind::Other`] should only be used
///   if there's no more specific option.
///
///     For example, a network timeout is a type of IO error, however it should
///     use [`Kind::Timeout`] rather than [`Kind::Io`].
///
/// - [`Kind::Invalid`] should be used when the input will never be valid (at
///   least in this version of the software), rather than input which is invalid
///   because of the current system state.
///
///     For example, an unknown identifier should use [`Kind::NotFound`] (for
///     example `ockam_vault_core`'s `Secret`) rather than [`Kind::Invalid`],
///
/// - [`Kind::Cancelled`], [`Kind::Timeout`], and [`Kind::Shutdown`] all sound
///   similar, but:
///
///     - [`Kind::Timeout`] should be used to map operations which timeout
///       externally, such as network requests. These may succeed if retried.
///
///     - [`Kind::Shutdown`] should be used to indicate the operation failed due
///       to the node shutting down.
///
///     - [`Kind::Cancelled`] is used when a request to cancel the operation
///       comes in while the operation is in progress.
#[repr(u8)]
#[non_exhaustive]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Kind {
    /// Indicates that there is no way to determine a more specific kind.
    // Internal note: We should not emit this from within the
    // `build-trust/ockam` crates.
    Unknown = 0,

    /// Used for serious internal errors, including panics.
    ///
    /// This generally should not be used unless we'd accept a bug report for
    /// the error.
    Internal = 1,

    /// The input was fundamentally invalid.
    ///
    /// For example, this is appropriate to use when:
    ///
    /// - A null pointer being passed as input in the FFI.
    /// - A string is used as input which is not UTF-8.
    /// - Parse failures of various kinds, but be sure to include more specific
    ///   information in the [`Error`](crate::Error) payload.
    ///
    /// Note that it is not appropriate for input which is invalid only due to
    /// the current system state.
    // Internal note: Check if there's a more specific `Kind` before using this
    // — it should mostly be `Origin
    Invalid = 2,

    /// The requested operation is not supported/implemented by that component.
    ///
    /// For example, this is appropriate for a component (for example, a custom
    /// Vault) which do not implement the entire API surface.
    Unsupported = 3,

    /// Some referenced entity was not found.
    ///
    /// For example, this may be appropriate for:
    ///
    /// - A vault which does not recognize a `Secret` which it recieves.
    /// - FFI that recieves an integer `handle` that does not belong to any
    ///   known entity.
    /// - Local [`Address`](crate::Address) which don't correspond to any known
    ///   `Worker` or `Processor`.
    /// - [`Address`](crate::Address) with a transport of an unknown or
    ///   unsupported type.
    ///
    /// Information about what exactly it is that could not be located should be
    /// available on the [`Error`](crate::Error) itself.
    NotFound = 4,

    /// The operation failed because
    ///
    /// For example, this may be appropriate for:
    ///
    /// - A vault which does not recognize a `Secret` which it recieves.
    /// - FFI that recieves an integer `handle` that does not belong to any
    ///   known entity.
    /// - Local [`Address`](crate::Address) which don't correspond to any known
    ///   `Worker` or `Processor`.
    /// - [`Address`](crate::Address) with a transport of an unknown or
    ///   unsupported type.
    ///
    /// Information about what exactly it is that could not be located should be
    /// available on the [`Error`](crate::Error) itself.
    AlreadyExists = 5,

    /// Indicates that some resource has been exhausted, or would be exhausted
    /// if the request were to be fulfilled.
    ///
    /// The resource in question could be memory, open file descriptors,
    /// storage, quota, simultaneous in-flight messages or tasks...
    ///
    /// Information about which resource it was that was exhausted should be
    /// available on the [`Error`](crate::Error) itself.
    ResourceExhausted = 6,

    /// An API was misused in some unspecific fashion.
    ///
    /// This is mostly intended for FFI and other non-Rust bindings — for
    /// example, it would be appropriate to map [`core::cell::BorrowError`] to
    /// this.
    // Internal note: Check if there's a more specific `Kind` before using this.
    Misuse = 7,

    /// Indicates the operation failed due to a cancellation request.
    ///
    /// See the type documentation on the difference between this,
    /// [`Kind::Shutdown`] and [`Kind::Timeout`].
    Cancelled = 8,

    /// Indicates that the operation failed due to the node shutting down.
    ///
    /// See the type documentation on the difference between this,
    /// [`Kind::Cancelled`] and [`Kind::Timeout`].
    Shutdown = 9,

    /// Indicates that the operation failed due to an external operation timing
    /// out, such as a network request, which may succeed if retried.
    ///
    /// See the type documentation on the difference between this,
    /// [`Kind::Shutdown`] and [`Kind::Cancelled`].
    Timeout = 10,

    /// Indicates an operation failed due to simultaneous attempts to modify a
    /// resource.
    Conflict = 11,

    /// Indicates an a failure to deserialize a message (or in rare cases,
    /// failure to serialize).
    Serialization = 12,

    /// Indicates some other I/O error.
    ///
    /// Specifics should be available on error payload.
    // Internal note: Check if there's a more specific `Kind` before using this.
    Io = 13,

    /// Indicates some other I/O error.
    ///
    /// Specifics should be available on error payload.
    // Internal note: Check if there's a more specific `Kind` before using this.
    Protocol = 14,

    /// Indicates an error that
    ///
    /// Specifics should be available on error payload.
    // Internal note: Check if there's a more specific `Kind` before using this.
    Other = 15,
    // This is a `#[non_exhaustive]` enum — we're free to add more variants
    // here. Do not add any which contain payloads (it should stay a "C style
    // enum"). Payload information should be added to the error itself.
    //
    // That said, until we finish migrating over to this, it's expected that
    // we'll need to add several new variants to all of these.
}

// Helper macro for converting a number into an enum variant with that value.
// Variants do not need to be contiguous. Requires listing the error variants
// again, but forces a compile-time error if the list is missing a variant.
macro_rules! from_prim {
    ($prim:expr => $Enum:ident { $($Variant:ident),* $(,)? }) => {{
        // Force a compile error if the list gets out of date.
        const _: fn(e: $Enum) = |e: $Enum| match e {
            $($Enum::$Variant => ()),*
        };
        match $prim {
            $(v if v == ($Enum::$Variant as _) => Some($Enum::$Variant),)*
            _ => None,
        }
    }}
}

impl Origin {
    /// Attempt to convert a numeric value into an `Origin`.
    ///
    /// `From<u8>` is also implemented, replacing unknown inputs with
    /// `Self::Unknown`.
    #[track_caller]
    pub fn from_u8(n: u8) -> Option<Self> {
        from_prim!(n => Origin {
            Unknown,
            Application,
            Vault,
            Transport,
            Node,
            Api,
            Identity,
            Channel,
            KeyExchange,
            Executor,
            Core,
            Ockam,
            Authorization,
            Other,
        })
    }
}

impl From<u8> for Origin {
    #[track_caller]
    fn from(src: u8) -> Self {
        match Self::from_u8(src) {
            Some(n) => n,
            None => {
                warn!("Unknown error origin: {}", src);
                Self::Unknown
            }
        }
    }
}

impl Kind {
    /// Attempt to construct a `Kind` from the numeric value.
    pub fn from_u8(n: u8) -> Option<Self> {
        from_prim!(n => Kind {
            Unknown,
            Internal,
            Invalid,
            Unsupported,
            NotFound,
            AlreadyExists,
            ResourceExhausted,
            Misuse,
            Cancelled,
            Shutdown,
            Timeout,
            Conflict,
            Io,
            Protocol,
            Serialization,
            Other
        })
    }
}

impl From<u8> for Kind {
    #[track_caller]
    fn from(src: u8) -> Self {
        match Self::from_u8(src) {
            Some(n) => n,
            None => {
                warn!("Unknown error origin: {}", src);
                Self::Unknown
            }
        }
    }
}

impl core::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // Kind of halfway between debug and display, TBH, but it's only used in
        // the Error debug output.
        write!(f, "[Origin::{:?}; Kind::{:?}", self.origin, self.kind,)?;
        if self.extra != 0 {
            write!(f, "; code = {}]", self.extra)
        } else {
            write!(f, "]")
        }
    }
}
