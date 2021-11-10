#[derive(Clone, Copy, Debug)]
pub enum EntityError {
    BareError = 1,
    VerifyFailed,
    InvalidInternalState,
    InvalidProof,
    ConsistencyError,
    ComplexEventsAreNotSupported,
    EventIdDoesNotMatch,
    ProfileIdDoesNotMatch,
    EmptyChange,
    ContactNotFound,
    EventNotFound,
    InvalidChainSequence,
    InvalidEventId,
    AttestationRequesterDoesNotMatch,
    AttestationNonceDoesNotMatch,
    InvalidHubResponse,
    InvalidParameter,
    SecureChannelVerificationFailed,
    SecureChannelTrustCheckFailed,
    SecureChannelCannotBeAuthenticated,
    ProfileInvalidResponseType,
    ProfileNotFound,
    NotImplemented,
    UnknownChannelMsgDestination,
    UnknownChannelMsgOrigin,
    InvalidLocalInfoType,
    InvalidSecureChannelInternalState,
    IdentityApiFailed,
    ContactVerificationFailed,
    InvalidProfileId,
    DuplicateCredential,
    CredentialNotFound,
    InvalidIssueState,
    CredentialTrustCheckFailed,
    SchemaIdDoesNotMatch,
    IssuerListenerInvalidMessage,
    HolderInvalidMessage,
    IssuerInvalidMessage,
    PresenterInvalidMessage,
    VerifierInvalidMessage,
}

impl EntityError {
    /// Integer code associated with the error domain.
    pub const DOMAIN_CODE: u32 = 20_000;
    /// Descriptive name for the error domain.
    pub const DOMAIN_NAME: &'static str = "OCKAM_ENTITY";
}

impl From<EntityError> for ockam_core::Error {
    fn from(e: EntityError) -> ockam_core::Error {
        ockam_core::Error::new(
            EntityError::DOMAIN_CODE + (e as u32),
            EntityError::DOMAIN_NAME,
        )
    }
}
