use ockam_core::Error;

/// Represents the failures that can occur in
/// an Ockam XX Key Agreement
#[derive(Clone, Copy, Debug)]
pub enum XXError {
    None,
    InvalidState,
    InternalVaultError,
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
        Self::new(XXError::DOMAIN_CODE + (err as u32), XXError::DOMAIN_NAME)
    }
}
