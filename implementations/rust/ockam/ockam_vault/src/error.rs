use crate::SecretType;
use ockam_core::{
    errcode::{Kind, Origin},
    Error,
};

/// Represents the failures that can occur in
/// an Ockam vault
#[derive(Clone, Debug)]
pub enum VaultError {
    /// Public key is invalid
    InvalidPublicKey,
    /// Unknown ECDH key type
    UnknownEcdhKeyType,
    /// Invalid key type
    InvalidKeyType,
    /// Key wasn't found
    KeyNotFound,
    /// Invalid Secret length
    InvalidSecretLength(SecretType, usize, u32),
    /// Invalid Public Key Length
    InvalidPublicLength,
    /// Invalid HKDF output type
    InvalidHkdfOutputType,
    /// AES encryption failed
    AeadAesGcmEncrypt,
    /// AES decryption failed
    AeadAesGcmDecrypt,
    /// HKDF key expansion failed
    HkdfExpandError,
    /// Invalid Sha256 Output length
    InvalidSha256Len,
    /// Invalid Signature Size
    InvalidSignatureSize,
}

impl ockam_core::compat::error::Error for VaultError {}
impl core::fmt::Display for VaultError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidPublicKey => write!(f, "public key is invalid"),
            Self::UnknownEcdhKeyType => write!(f, "unknown ECDH key type"),
            Self::InvalidKeyType => write!(f, "invalid key type"),
            Self::InvalidSecretLength(secret_type, actual, expected) => write!(
                f,
                "invalid secret length for {}. Actual: {}, Expected: {}",
                secret_type, actual, expected
            ),
            Self::InvalidPublicLength => write!(f, "invalid public key length"),
            Self::InvalidHkdfOutputType => write!(f, "invalid HKDF output type"),
            Self::AeadAesGcmEncrypt => write!(f, "aes encryption failed"),
            Self::AeadAesGcmDecrypt => write!(f, "aes decryption failed"),
            Self::HkdfExpandError => write!(f, "hkdf key expansion failed"),
            Self::KeyNotFound => write!(f, "key not found"),
            Self::InvalidSha256Len => write!(f, "invalid sha256 len"),
            Self::InvalidSignatureSize => write!(f, "invalid signature len"),
        }
    }
}

impl From<VaultError> for Error {
    #[track_caller]
    fn from(err: VaultError) -> Self {
        use VaultError::*;
        let kind = match err {
            InvalidPublicKey | InvalidKeyType | InvalidHkdfOutputType => Kind::Misuse,
            UnknownEcdhKeyType => Kind::NotFound,
            _ => Kind::Invalid,
        };

        Error::new(Origin::Vault, kind, err)
    }
}
