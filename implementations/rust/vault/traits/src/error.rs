use ockam_common::error::OckamError;

/// Represents the failures that can occur in
/// an Ockam Vault trait
#[derive(Clone, Copy, Debug)]
pub enum Error {
    /// None
    None,
    /// An unknown secret type value was supplied
    UnknownSecretTypeValue,
    /// An unknown secret persistence value was supplied
    UnknownSecretPersistenceValue,
}

impl Error {
    /// Error domain
    pub const ERROR_DOMAIN: &'static str = "VAULT_TRAITS_ERROR_DOMAIN";
}

impl Into<OckamError> for Error {
    fn into(self) -> OckamError {
        OckamError::new(self as u32, Error::ERROR_DOMAIN)
    }
}
