use ockam_core::{
    errcode::{ErrorCode, Kind, Origin},
    thiserror, Error,
};
use std::ffi::CString;
use std::os::raw::c_char;

#[repr(C)]
#[derive(Debug, PartialEq)]
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
#[derive(Clone, Copy, Debug, thiserror::Error)]
pub enum FfiError {
    /// Persistence is not supported for this Vault implementation.
    #[error("persistence is not supported for this Vault implementation.")]
    VaultDoesNotSupportPersistence = 1,

    /// An underlying filesystem error prevented Vault creation.
    #[error("an underlying filesystem error prevented Vault creation.")]
    ErrorCreatingFilesystemVault,

    /// Invalid parameter.
    #[error("invalid parameter.")]
    InvalidParam,

    /// Entry not found.
    #[error("entry not found.")]
    EntryNotFound,

    /// Unknown public key type.
    #[error("unknown public key type.")]
    UnknownPublicKeyType,

    /// Invalid string.
    #[error("invalid string.")]
    InvalidString,

    /// Buffer is too small.
    #[error("buffer is too small.")]
    BufferTooSmall,

    /// A public key is invalid.
    #[error("a public key is invalid.")]
    InvalidPublicKey,

    /// No such Vault.
    #[error("no such Vault.")]
    VaultNotFound,

    /// Ownership error.
    #[error("ownership error.")]
    OwnershipError,

    /// Caught a panic (which would be UB if we let it unwind across the FFI).
    #[error("caught a panic (which would be UB if we let it unwind across the FFI).")]
    UnexpectedPanic,
}

impl From<FfiError> for Error {
    fn from(err: FfiError) -> Self {
        Error::new(ErrorCode::new(Origin::Other, Kind::Other), err)
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
