use ockam_core::{
    errcode::{Kind, Origin},
    Error,
};

/// Identity crate error
#[derive(Clone, Copy, Debug)]
pub enum IdentityError {
    /// Bare serialization error
    BareError = 1,
    /// `IdentityChangeHistory` verification failed
    VerifyFailed,
    /// Invalid internal state of the `Identity`
    InvalidInternalState,
    /// Consistency check failed
    ConsistencyError,
    /// SecureChannel signature check failed during Identity authentication
    SecureChannelVerificationFailed,
    /// SecureChannel `TrustPolicy` check failed
    SecureChannelTrustCheckFailed,
    /// Custom payload wasn't provided where required
    NoCustomPayload,
    /// Unknown channel message destination
    UnknownChannelMsgDestination,
    /// Invalid `LocalInfo` type
    InvalidLocalInfoType,
    /// Internal Secure Channel error
    InvalidSecureChannelInternalState,
    /// `Identity` verification error
    IdentityVerificationFailed,
    /// Invalid `IdentityIdentifier` format
    InvalidIdentityId,
    /// Invalid `Credential` format
    InvalidCredentialFormat,
    /// Unknown Authority
    UnknownAuthority,
    /// `Credential` verification failed
    CredentialVerificationFailed,
    /// SecureChannel with this address already exists
    DuplicateSecureChannel,
    /// Invalid nonce format
    InvalidNonce,
    /// Nonce overflow
    NonceOverflow,
    /// SecureChannel was not found in the Registry
    SecureChannelNotFound,
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
