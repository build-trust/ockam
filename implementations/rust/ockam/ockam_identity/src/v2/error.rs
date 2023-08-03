use ockam_core::{
    errcode::{Kind, Origin},
    Error,
};

/// Identity crate error
#[derive(Clone, Copy, Debug)]
pub enum IdentityError {
    /// Invalid key type
    InvalidKeyType = 1,
    /// Invalid Identifier format
    InvalidIdentifier,
    /// Identity Change History is empty
    EmptyIdentity,
    /// Identity Verification Failed
    IdentityVerificationFailed,
    /// PurposeKeyAttestation Verification Failed
    PurposeKeyAttestationVerificationFailed,
    /// Credential Verification Failed
    CredentialVerificationFailed,
    /// Error occurred while getting current UTC Timestamp
    UnknownTimestamp,
    /// Attributes were already set
    AttributesAlreadySet,
    /// Attributes hasn't been set
    AttributesNotSet,
    /// Schema was not yet set
    SchemaNotSet,
    /// Maximum time for credential validity exceeded
    CredentialTtlExceeded,
    /// Credential ttl wasn't set
    CredentialTtlNotSet,
    /// Unknown Authority
    UnknownAuthority,
    /// Unknown version of the Credential
    UnknownCredentialVersion,
    /// Unknown version of the PurposeKeyAttestation
    UnknownPurposeKeyVersion,
    /// Unknown version of the Identity
    UnknownIdentityVersion,
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
