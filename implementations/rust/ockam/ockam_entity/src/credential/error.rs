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
    /// Invalid public key provided
    InvalidPublicKey,
    /// The number of presentations does not match the number of manifests
    MismatchedPresentationAndManifests,
}

impl CredentialError {
    /// Integer code associated with the error domain.
    pub const DOMAIN_CODE: u32 = 33_000;

    #[cfg(any(feature = "std", feature = "alloc"))]
    /// Descriptive name for the error domain
    pub const DOMAIN_NAME: &'static str = "OCKAM_CREDENTIAL";

    pub(crate) const fn as_u32(self) -> u32 {
        match self {
            CredentialError::None => 0,
            CredentialError::MismatchedAttributesAndClaims => 100,
            CredentialError::MismatchedAttributeClaimType => 200,
            CredentialError::InvalidCredentialAttribute => 300,
            CredentialError::InvalidCredentialSchema => 400,
            CredentialError::InvalidCredentialOffer => 500,
            CredentialError::InvalidPresentationManifest => 600,
            CredentialError::InvalidPresentationChallenge => 700,
            CredentialError::InvalidCredentialPresentation(i) => 800u32 + i,
            CredentialError::InvalidPublicKey => 900,
            CredentialError::MismatchedPresentationAndManifests => 1000,
        }
    }
}

#[cfg(any(feature = "std", feature = "alloc"))]
impl From<CredentialError> for Error {
    fn from(v: CredentialError) -> Error {
        let t = v.as_u32();
        Error::new(
            CredentialError::DOMAIN_CODE + t,
            CredentialError::DOMAIN_NAME,
        )
    }
}

#[cfg(all(not(feature = "std"), not(feature = "alloc")))]
impl From<CredentialError> for Error {
    fn from(v: CredentialError) -> Error {
        Error::new(CredentialError::DOMAIN_CODE + v.as_u32())
    }
}
