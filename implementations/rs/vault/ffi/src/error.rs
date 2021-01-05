use ffi_support::{ErrorCode, ExternError};
use ockam_common::error::OckamError;

/// Represents the failures that can occur in
/// an Ockam FFI Vault
#[derive(Clone, Copy, Debug)]
pub enum Error {
    None,
    VaultDoesntSupportPersistence,
    InvalidParam,
    EntryNotFound,
    UnknownPublicKeyType,
    InvalidString,
    BufferTooSmall,
    InvalidPublicKey,
    VaultNotFound,
}

impl Error {
    /// Error domain
    pub const ERROR_DOMAIN: &'static str = "VAULT_FFI_ERROR_DOMAIN";
}

impl From<Error> for OckamError {
    fn from(err: Error) -> Self {
        OckamError::new(err as u32, Error::ERROR_DOMAIN)
    }
}

impl From<Error> for ExternError {
    fn from(err: Error) -> Self {
        ExternError::new_error(ErrorCode::new(err as i32), Error::ERROR_DOMAIN)
    }
}
