use ockam_core::{
    errcode::{Kind, Origin},
    Error,
};

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
    IdentityVerificationFailed,
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

impl ockam_core::compat::error::Error for IdentityError {}
impl core::fmt::Display for IdentityError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(self, f)
    }
}

impl From<IdentityError> for Error {
    #[track_caller]
    fn from(err: IdentityError) -> Self {
        let kind = Kind::Unknown; // FIXME: fill these in with more
                                  // meaningful error kinds
        Error::new(Origin::Identity, kind, err)
    }
}
