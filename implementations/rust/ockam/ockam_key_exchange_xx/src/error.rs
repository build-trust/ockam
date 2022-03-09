use ockam_core::Error;

/// Represents the failures that can occur in
/// an Ockam XX Key Agreement
#[derive(Clone, Copy, Debug)]
pub enum XXError {
    /// The key exchange protocol is in an invalid state.
    InvalidState = 1,
    /// An internal Vault error has occurred.
    InternalVaultError,
    /// A message had an unexpected length.
    MessageLenMismatch,
}

impl XXError {
    /// Integer code associated with the error domain.
    pub const DOMAIN_CODE: u32 = 14_000;
    /// Descriptive name for the error domain.
    pub const DOMAIN_NAME: &'static str = "OCKAM_KEX_XX";
}

impl From<XXError> for Error {
    fn from(err: XXError) -> Self {
        Self::new(
            XXError::DOMAIN_CODE + (err as u32),
            ockam_core::compat::format!("{}::{:?}", module_path!(), err),
        )
    }
}
