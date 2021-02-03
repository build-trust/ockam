/// Represents the failures that can occur in
/// an Ockam Software trait
#[derive(Clone, Copy, Debug)]
pub enum Error {
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

impl Error {
    /// Error domain
    pub const ERROR_DOMAIN: &'static str = "VAULT_SOFTWARE_ERROR_DOMAIN";
}

impl Into<ockam_core::Error> for Error {
    fn into(self) -> ockam_core::Error {
        ockam_core::Error::new(self as u32, Error::ERROR_DOMAIN)
    }
}
