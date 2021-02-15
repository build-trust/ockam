use ockam_core::Error;

/// The error types that can occur when creating or verifying
/// a credential.
#[derive(Clone, Copy, Debug)]
pub enum CredentialError {
    /// No error
    None,
    /// Mismatched number of attributes in schema and provided claims to be signed
    MismatchedAttributesAndClaims,
    /// Mismatched attribute type and provided claim
    MismatchedAttributeClaimType,
    /// Data that cannot be converted to a claim
    InvalidCredentialAttribute,
    /// A schema with no attributes
    InvalidCredentialSchema,
    /// Invalid Credential offer
    InvalidCredentialOffer,
    /// A manifest that requests to reveal a bad credential attribute
    InvalidPresentationManifest,
    /// An challenge calculation was different than expected
    InvalidPresentationChallenge,
    /// Returns the index of the first failed credential presentation
    InvalidCredentialPresentation(u32),
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
        let t = match v {
            CredentialError::None => 0,
            CredentialError::MismatchedAttributesAndClaims => 1000,
            CredentialError::MismatchedAttributeClaimType => 2000,
            CredentialError::InvalidCredentialAttribute => 3000,
            CredentialError::InvalidCredentialSchema => 4000,
            CredentialError::InvalidCredentialOffer => 5000,
            CredentialError::InvalidPresentationManifest => 6000,
            CredentialError::InvalidPresentationChallenge => 7000,
            CredentialError::InvalidCredentialPresentation(i) => 8000u32 + i,
        };
        Error::new(
            CredentialError::DOMAIN_CODE + t,
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
