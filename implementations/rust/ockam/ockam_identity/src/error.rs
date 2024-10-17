use ockam_core::compat::string::String;
use ockam_core::{
    errcode::{Kind, Origin},
    Error,
};

/// Identity crate error
#[repr(u8)]
#[derive(Clone, Debug)]
pub enum IdentityError {
    /// Invalid key type
    InvalidKeyType = 1,
    /// Invalid Key Data
    InvalidKeyData,
    /// Invalid Identifier format
    InvalidIdentifier(String),
    /// Identity Change History is empty
    EmptyIdentity,
    /// Identity Verification Failed
    IdentityVerificationFailed,
    /// PurposeKeyAttestation Verification Failed
    PurposeKeyAttestationVerificationFailed,
    /// Credential Verification Failed
    CredentialVerificationFailed,
    /// Unknown Authority
    UnknownAuthority,
    /// No CredentialRetriever
    NoCredentialRetriever,
    /// Unknown version of the Credential
    UnknownCredentialVersion,
    /// Invalid data_type value for Credential
    InvalidCredentialDataType,
    /// Unknown version of the Identity
    UnknownIdentityVersion,
    /// Invalid data_type value for Identity
    InvalidIdentityDataType,
    /// Unknown version of the PurposeKeyAttestation
    UnknownPurposeKeyAttestationVersion,
    /// Invalid data_type value for PurposeKeyAttestation
    InvalidPurposeKeyAttestationDataType,
    /// SecureChannelTrustCheckFailed
    SecureChannelTrustCheckFailed,
    /// Invalid Nonce value
    InvalidNonce,
    /// Nonce overflow
    NonceOverflow,
    /// Unknown message destination
    UnknownChannelMsgDestination,
    /// Duplicate Secure Channel
    DuplicateSecureChannel,
    /// Consistency Error
    ConsistencyError,
    /// Secret Key doesn't correspond to the Identity
    WrongSecretKey,
    /// CredentialRetriever was already set
    CredentialRetrieverCreatorAlreadySet,
    /// Credential is missing in the cache
    CachedCredentialMissing,
    /// Given address is already a subscriber for that RemoteCredentialRetriever
    AddressAlreadySubscribedForThatCredentialRetriever,
    /// Given address hasn't been subscribed for that RemoteCredentialRetriever
    AddressIsNotSubscribedForThatCredentialRetriever,
    /// Credential retriever couldn't return a credential
    NoCredential,
    /// Persistence is currently only supported for key exchange only channels
    PersistentSupportIsLimited,
    /// Secure Channel not found in the storage
    PersistentSecureChannelNotFound,
    /// Unknown Secure Channel Role value
    UnknownRole,
    /// Handshake ended up in an internal invalid state
    HandshakeInternalError,
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
