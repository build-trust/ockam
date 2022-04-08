#![deny(missing_docs)]
#![allow(missing_docs)] // Contents are self describing for now.

use ockam_core::{
    errcode::{Kind, Origin},
    Error,
};

/// An enumeration of different error types emitted by this library.
///
/// Most user code should use [`crate::Error`] instead.
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
    NoSuchProtocol,
    SystemAddressNotBound,
    SystemInvalidConfiguration,
}

impl ockam_core::compat::error::Error for OckamError {}
impl core::fmt::Display for OckamError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(self, f)
    }
}

impl From<OckamError> for Error {
    fn from(err: OckamError) -> Error {
        use OckamError::*;
        // TODO: improve this mapping
        let kind = match err {
            SystemAddressNotBound | SystemInvalidConfiguration | InvalidParameter => Kind::Misuse,
            _ => Kind::Protocol,
        };

        Error::new(Origin::Ockam, kind, err)
    }
}
