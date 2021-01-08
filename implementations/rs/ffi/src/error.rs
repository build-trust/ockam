use ockam_common::error::OckamError;
use std::os::raw::c_char;

#[repr(C)]
#[derive(Debug, PartialEq)]
/// Ffi error
pub struct FfiOckamError {
    code: i32,
    domain: *const c_char,
}

impl FfiOckamError {
    /// Create new error
    pub fn new(code: i32, domain: &'static str) -> Self {
        Self {
            code,
            domain: domain.as_ptr() as *const c_char,
        }
    }

    /// No error
    pub fn none() -> Self {
        Self {
            code: 0,
            domain: std::ptr::null(),
        }
    }
}

impl From<OckamError> for FfiOckamError {
    fn from(err: OckamError) -> Self {
        Self::new(err.code() as i32, err.domain())
    }
}

/// Represents the failures that can occur in
/// an Ockam FFI Vault
#[derive(Clone, Copy, Debug)]
pub enum Error {
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

impl Error {
    /// Error domain
    pub const ERROR_DOMAIN: &'static str = "FFI_ERROR_DOMAIN";
}

impl From<Error> for OckamError {
    fn from(err: Error) -> Self {
        OckamError::new(err as u32, Error::ERROR_DOMAIN)
    }
}

impl From<Error> for FfiOckamError {
    fn from(err: Error) -> Self {
        Self::new(err as i32, Error::ERROR_DOMAIN)
    }
}
