#![deny(missing_docs)]
#![allow(missing_docs)] // Contents are self describing for now.

use ockam_core::{
    errcode::{ErrorCode, Kind, Origin},
    thiserror, Error2,
};

/// An enumeration of different error types emitted by this library.
///
/// Most user code should use [`crate::Error`] instead.
#[derive(Clone, Copy, Debug, thiserror::Error)]
pub enum OckamError {
    #[error("BareError")]
    BareError = 1,
    #[error("VerifyFailed")]
    VerifyFailed,
    #[error("InvalidInternalState")]
    InvalidInternalState,
    #[error("InvalidProof")]
    InvalidProof,
    #[error("ConsistencyError")]
    ConsistencyError, // 5
    #[error("ComplexEventsAreNotSupported")]
    ComplexEventsAreNotSupported,
    #[error("EventIdDoesNotMatch")]
    EventIdDoesNotMatch,
    #[error("IdentityIdDoesNotMatch")]
    IdentityIdDoesNotMatch,
    #[error("EmptyChange")]
    EmptyChange,
    #[error("ContactNotFound")]
    ContactNotFound, // 10
    #[error("EventNotFound")]
    EventNotFound,
    #[error("InvalidChainSequence")]
    InvalidChainSequence,
    #[error("InvalidEventId")]
    InvalidEventId,
    #[error("AttestationRequesterDoesNotMatch")]
    AttestationRequesterDoesNotMatch,
    #[error("AttestationNonceDoesNotMatch")]
    AttestationNonceDoesNotMatch, // 15
    #[error("InvalidHubResponse")]
    InvalidHubResponse,
    #[error("InvalidParameter")]
    InvalidParameter,
    #[error("SecureChannelVerificationFailed")]
    SecureChannelVerificationFailed,
    #[error("SecureChannelCannotBeAuthenticated")]
    SecureChannelCannotBeAuthenticated,
    #[error("NoSuchProtocol")]
    NoSuchProtocol, // 20
    #[error("SystemAddressNotBound")]
    SystemAddressNotBound,
    #[error("SystemInvalidConfiguration")]
    SystemInvalidConfiguration,
}

impl From<OckamError> for Error2 {
    fn from(err: OckamError) -> Error2 {
        use OckamError::*;
        // TODO: improve this mapping
        let kind = match err {
            SystemAddressNotBound | SystemInvalidConfiguration | InvalidParameter => Kind::Misuse,
            _ => Kind::Protocol,
        };

        Error2::new(ErrorCode::new(Origin::Ockam, kind), err)
    }
}
