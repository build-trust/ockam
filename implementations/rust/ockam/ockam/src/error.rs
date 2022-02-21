#![deny(missing_docs)]
#![allow(missing_docs)] // Contents are self describing for now.
#[derive(Clone, Copy, Debug)]
pub enum OckamError {
    BareError = 1,
    VerifyFailed,
    InvalidInternalState,
    InvalidProof,
    ConsistencyError, // 5
    ComplexEventsAreNotSupported,
    EventIdDoesNotMatch,
    IdentityIdDoesNotMatch,
    EmptyChange,
    ContactNotFound, // 10
    EventNotFound,
    InvalidChainSequence,
    InvalidEventId,
    AttestationRequesterDoesNotMatch,
    AttestationNonceDoesNotMatch, // 15
    InvalidHubResponse,
    InvalidParameter,
    SecureChannelVerificationFailed,
    SecureChannelCannotBeAuthenticated,
    NoSuchProtocol, // 20
    SystemAddressNotBound,
    SystemInvalidConfiguration,
}

impl OckamError {
    /// Integer code associated with the error domain.
    pub const DOMAIN_CODE: u32 = 10_000;
    /// Descriptive name for the error domain.
    pub const DOMAIN_NAME: &'static str = "OCKAM";
}

impl From<OckamError> for ockam_core::Error {
    fn from(e: OckamError) -> ockam_core::Error {
        ockam_core::Error::new(
            OckamError::DOMAIN_CODE + (e as u32),
            OckamError::DOMAIN_NAME,
        )
    }
}
