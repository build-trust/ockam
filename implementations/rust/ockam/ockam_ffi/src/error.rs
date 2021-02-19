use ockam_core::Error;
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

impl From<Error> for FfiOckamError {
    fn from(err: Error) -> Self {
        // TODO: Should this graciously fail?
        let heaped = CString::new(err.domain().as_bytes()).unwrap();

        // Past this point C is responsible for freeing up its memory!
        Self {
            code: err.code() as i32,
            domain: heaped.into_raw(),
        }
    }
}

/// Represents the failures that can occur in an Ockam FFI Vault.
#[derive(Clone, Copy, Debug)]
pub enum FfiError {
    None,
    VaultDoesntSupportPersistence,
    ErrorCreatingFilesystemVault,
    InvalidParam,
    EntryNotFound,
    UnknownPublicKeyType,
    InvalidString,
    BufferTooSmall,
    InvalidPublicKey,
    VaultNotFound,
    OwnershipError,
}

impl FfiError {
    /// Integer code associated with the error domain.
    pub const DOMAIN_CODE: u32 = 13_000;
    /// Descriptive name for the error domain.
    pub const DOMAIN_NAME: &'static str = "OCKAM_FFI";
}

impl From<FfiError> for Error {
    fn from(err: FfiError) -> Self {
        Self::new(FfiError::DOMAIN_CODE + (err as u32), FfiError::DOMAIN_NAME)
    }
}

impl From<FfiError> for FfiOckamError {
    fn from(err: FfiError) -> Self {
        let err: Error = err.into();
        Self::from(err)
    }
}
