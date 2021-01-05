use ockam_common::error::OckamError;

/// Represents the failures that can occur in
/// an Ockam Filesystem Vault
#[derive(Clone, Copy, Debug)]
pub enum Error {
    None,
    SecretFromAnotherVault,
    InvalidSecret,
    IOError,
    InvalidPersistenceId,
    EntryNotFound,
}

impl Error {
    /// Error domain
    pub const ERROR_DOMAIN: &'static str = "VAULT_FILESYSTEM_ERROR_DOMAIN";
}

impl Into<OckamError> for Error {
    fn into(self) -> OckamError {
        OckamError::new(self as u32, Error::ERROR_DOMAIN)
    }
}
