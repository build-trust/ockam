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

impl Into<Error> for VaultError {
    fn into(self) -> Error {
        Error::new(Self::DOMAIN_CODE + (self as u32), Self::DOMAIN_NAME)
    }
}
