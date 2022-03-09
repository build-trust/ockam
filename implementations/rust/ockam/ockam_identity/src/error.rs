#[derive(Clone, Copy, Debug)]
pub enum IdentityError {
    BareError = 1,
    VerifyFailed,
    InvalidInternalState,
    InvalidProof,
    ConsistencyError,
    ComplexEventsAreNotSupported,
    EventIdDoesNotMatch,
    IdentityIdDoesNotMatch,
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
    IdentityInvalidResponseType,
    IdentityNotFound,
    NotImplemented,
    UnknownChannelMsgDestination,
    UnknownChannelMsgOrigin,
    InvalidLocalInfoType,
    InvalidSecureChannelInternalState,
    ContactVerificationFailed,
    InvalidIdentityId,
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

impl IdentityError {
    /// Integer code associated with the error domain.
    pub const DOMAIN_CODE: u32 = 20_000;
    /// Descriptive name for the error domain.
    pub const DOMAIN_NAME: &'static str = "OCKAM_ENTITY";
}

impl From<IdentityError> for ockam_core::Error {
    fn from(e: IdentityError) -> ockam_core::Error {
        ockam_core::Error::new(
            IdentityError::DOMAIN_CODE + (e as u32),
            ockam_core::compat::format!("{}::{:?}", module_path!(), e),
        )
    }
}
