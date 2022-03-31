use ockam_core::{
    errcode::{ErrorCode, Kind, Origin},
    thiserror, Error2,
};

#[derive(Clone, Copy, Debug, thiserror::Error)]
pub enum IdentityError {
    #[error("BareError")]
    BareError = 1,
    #[error("VerifyFailed")]
    VerifyFailed,
    #[error("InvalidInternalState")]
    InvalidInternalState,
    #[error("InvalidProof")]
    InvalidProof,
    #[error("ConsistencyError")]
    ConsistencyError,
    #[error("ComplexEventsAreNotSupported")]
    ComplexEventsAreNotSupported,
    #[error("EventIdDoesNotMatch")]
    EventIdDoesNotMatch,
    #[error("IdentityIdDoesNotMatch")]
    IdentityIdDoesNotMatch,
    #[error("EmptyChange")]
    EmptyChange,
    #[error("ContactNotFound")]
    ContactNotFound,
    #[error("EventNotFound")]
    EventNotFound,
    #[error("InvalidChainSequence")]
    InvalidChainSequence,
    #[error("InvalidEventId")]
    InvalidEventId,
    #[error("AttestationRequesterDoesNotMatch")]
    AttestationRequesterDoesNotMatch,
    #[error("AttestationNonceDoesNotMatch")]
    AttestationNonceDoesNotMatch,
    #[error("InvalidHubResponse")]
    InvalidHubResponse,
    #[error("InvalidParameter")]
    InvalidParameter,
    #[error("SecureChannelVerificationFailed")]
    SecureChannelVerificationFailed,
    #[error("SecureChannelTrustCheckFailed")]
    SecureChannelTrustCheckFailed,
    #[error("SecureChannelCannotBeAuthenticated")]
    SecureChannelCannotBeAuthenticated,
    #[error("IdentityInvalidResponseType")]
    IdentityInvalidResponseType,
    #[error("IdentityNotFound")]
    IdentityNotFound,
    #[error("NotImplemented")]
    NotImplemented,
    #[error("UnknownChannelMsgDestination")]
    UnknownChannelMsgDestination,
    #[error("UnknownChannelMsgOrigin")]
    UnknownChannelMsgOrigin,
    #[error("InvalidLocalInfoType")]
    InvalidLocalInfoType,
    #[error("InvalidSecureChannelInternalState")]
    InvalidSecureChannelInternalState,
    #[error("ContactVerificationFailed")]
    ContactVerificationFailed,
    #[error("InvalidIdentityId")]
    InvalidIdentityId,
    #[error("DuplicateCredential")]
    DuplicateCredential,
    #[error("CredentialNotFound")]
    CredentialNotFound,
    #[error("InvalidIssueState")]
    InvalidIssueState,
    #[error("CredentialTrustCheckFailed")]
    CredentialTrustCheckFailed,
    #[error("SchemaIdDoesNotMatch")]
    SchemaIdDoesNotMatch,
    #[error("IssuerListenerInvalidMessage")]
    IssuerListenerInvalidMessage,
    #[error("HolderInvalidMessage")]
    HolderInvalidMessage,
    #[error("IssuerInvalidMessage")]
    IssuerInvalidMessage,
    #[error("PresenterInvalidMessage")]
    PresenterInvalidMessage,
    #[error("VerifierInvalidMessage")]
    VerifierInvalidMessage,
}

impl From<IdentityError> for Error2 {
    fn from(err: IdentityError) -> Self {
        let kind = Kind::Unknown; // FIXME: fill these in with more
                                  // meaningful error kinds
        Error2::new(ErrorCode::new(Origin::Identity, kind), err)
    }
}
