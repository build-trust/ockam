#[derive(Clone, Copy, Debug)]
pub enum OckamError {
    None,
    BareError,
    VerifyFailed,
    InvalidInternalState,
    InvalidProof,
    ConsistencyError,
    ComplexEventsAreNotSupported,
    EventIdDoesntMatch,
    ProfileIdDoesntMatch,
    EmptyChange,
    ContactNotFound,
    EventNotFound,
    InvalidChainSequence,
    InvalidEventId,
    AttestationRequesterDoesntMatch,
    AttestationNonceDoesntMatch,
    InvalidHubResponse,
    InvalidParameter,
    SecureChannelVerificationFailed,
    SecureChannelCannotBeAuthenticated,
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
