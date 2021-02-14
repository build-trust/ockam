use ockam_core::Error;

/// Represents the failures that can occur in
/// an Ockam vault
#[derive(Clone, Copy, Debug)]
pub enum VaultError {
    None,
    SecretFromAnotherVault,
    InvalidPublicKey,
    Ecdh,
    UnknownEcdhKeyType,
    InvalidKeyType,
    EntryNotFound,
    InvalidAesKeyLength,
    InvalidHkdfOutputType,
    InvalidPrivateKeyLen,
    AeadAesGcmEncrypt,
    AeadAesGcmDecrypt,
    InvalidSignature,
    HkdfExpandError,
    SecretNotFound,
}

impl VaultError {
    /// Integer code associated with the error domain.
    pub const DOMAIN_CODE: u32 = 12_000;
    /// Descriptive name for the error domain.
    pub const DOMAIN_NAME: &'static str = "OCKAM_VAULT";
}

impl From<VaultError> for Error {
    fn from(err: VaultError) -> Self {
        Self::new(
            VaultError::DOMAIN_CODE + (err as u32),
            VaultError::DOMAIN_NAME,
        )
    }
}
