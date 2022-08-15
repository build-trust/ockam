use ockam_core::{
    errcode::{Kind, Origin},
    Error,
};
use std::ffi::CString;
use std::os::raw::c_char;

#[repr(C)]
#[derive(Debug, PartialEq, Eq)]
/// Error type relating to FFI specific failures.
pub struct FfiOckamError {
    code: i32,
    domain: *const c_char,
}

impl FfiOckamError {
    /// Create a new error.
    pub fn new(code: i32, domain: &str) -> Self {
        Self {
            code,
            // TODO: Should this graciously fail?
            domain: CString::new(domain.as_bytes()).unwrap().into_raw(),
        }
    }

    /// No error.
    pub fn none() -> Self {
        Self {
            code: 0,
            domain: std::ptr::null(),
        }
    }
}

/// Represents the failures that can occur in an Ockam FFI Vault.
#[derive(Clone, Copy, Debug)]
pub enum FfiError {
    /// Persistence is not supported for this Vault implementation.
    VaultDoesNotSupportPersistence = 1,

    /// An underlying filesystem error prevented Vault creation.
    ErrorCreatingFilesystemVault,

    /// Invalid parameter.
    InvalidParam,

    /// Entry not found.
    EntryNotFound,

    /// Unknown public key type.
    UnknownPublicKeyType,

    /// Invalid string.
    InvalidString,

    /// Buffer is too small.
    BufferTooSmall,

    /// A public key is invalid.
    InvalidPublicKey,

    /// No such Vault.
    VaultNotFound,

    /// Ownership error.
    OwnershipError,

    /// Caught a panic (which would be UB if we let it unwind across the FFI).
    UnexpectedPanic,
}
impl ockam_core::compat::error::Error for FfiError {}
impl From<FfiError> for Error {
    #[track_caller]
    fn from(err: FfiError) -> Self {
        Error::new(Origin::Other, Kind::Other, err)
    }
}
impl core::fmt::Display for FfiError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::VaultDoesNotSupportPersistence => write!(
                f,
                "persistence is not supported for this Vault implementation."
            ),
            Self::ErrorCreatingFilesystemVault => write!(
                f,
                "an underlying filesystem error prevented Vault creation."
            ),
            Self::InvalidParam => write!(f, "invalid parameter."),
            Self::EntryNotFound => write!(f, "entry not found."),
            Self::UnknownPublicKeyType => write!(f, "unknown public key type."),
            Self::InvalidString => write!(f, "invalid string."),
            Self::BufferTooSmall => write!(f, "buffer is too small."),
            Self::InvalidPublicKey => write!(f, "a public key is invalid."),
            Self::VaultNotFound => write!(f, "no such Vault."),
            Self::OwnershipError => write!(f, "ownership error."),
            Self::UnexpectedPanic => write!(
                f,
                "caught a panic (which would be UB if we let it unwind across the FFI)."
            ),
        }
    }
}

impl From<Error> for FfiOckamError {
    fn from(err: Error) -> Self {
        Self::new(
            err.code().origin as i32 * 10_000 + err.code().kind as i32,
            "unknown",
        )
    }
}

impl From<FfiError> for FfiOckamError {
    fn from(err: FfiError) -> Self {
        let err2: Error = err.into();
        Self::from(err2)
    }
}

/// # Safety
/// frees `FfiOckamError::domain` if it's non-null
#[no_mangle]
pub unsafe extern "C" fn ockam_vault_free_error(context: &mut FfiOckamError) {
    if !context.domain.is_null() {
        let _ = CString::from_raw(context.domain as *mut _);
        context.domain = core::ptr::null();
    }
}
