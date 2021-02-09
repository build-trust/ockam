use ockam_core::Error;

/// The error types that can occur when creating or verifying
/// a credential.
#[derive(Clone, Copy, Debug)]
pub enum CredentialError {
    /// No error
    None,
}

impl CredentialError {
    /// Integer code associated with the error domain.
    pub const DOMAIN_CODE: u32 = 33_000;

    #[cfg(feature = "std")]
    /// Descriptive name for the error domain
    pub const DOMAIN_NAME: &'static str = "OCKAM_CREDENTIAL";
}

#[cfg(feature = "std")]
impl From<CredentialError> for Error {
    fn from(v: CredentialError) -> Error {
        Error::new(
            CredentialError::DOMAIN_CODE + (v as u32),
            CredentialError::DOMAIN_NAME,
        )
    }
}

#[cfg(not(feature = "std"))]
impl From<CredentialError> for Error {
    fn from(v: CredentialError) -> Error {
        Error::new(CredentialError::DOMAIN_CODE + (v as u32))
    }
}
