use crate::SecretType;
use ockam_core::compat::string::String;
use ockam_core::{
    errcode::{Kind, Origin},
    Error,
};

/// Represents the failures that can occur in
/// an Ockam vault
#[derive(Clone, Debug)]
pub enum VaultError {
    /// Secret does not belong to this vault
    SecretFromAnotherVault,
    /// Public key is invalid
    InvalidPublicKey,
    /// Unknown ECDH key type
    UnknownEcdhKeyType,
    /// Invalid key type
    InvalidKeyType,
    /// Entry not found
    EntryNotFound(String),
    /// Invalid AES key length
    InvalidAesKeyLength,
    /// Invalid Secret length
    InvalidSecretLength(SecretType, usize, u32),
    /// Invalid HKDF output type
    InvalidHkdfOutputType,
    /// Invalid private key length
    InvalidPrivateKeyLen,
    /// AES encryption failed
    AeadAesGcmEncrypt,
    /// AES decryption failed
    AeadAesGcmDecrypt,
    /// HKDF key expansion failed
    HkdfExpandError,
    /// Secret not found
    SecretNotFound,
    /// Invalid X25519 secret length
    InvalidX25519SecretLength,
    /// Invalid Ed25519 secret
    InvalidEd25519Secret,
    /// Invalid Secret Attributes
    InvalidSecretAttributes,
    /// IO error
    StorageError,
    /// Invalid Storage data
    InvalidStorageData,
}

impl ockam_core::compat::error::Error for VaultError {}
impl core::fmt::Display for VaultError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::SecretFromAnotherVault => write!(f, "secret does not belong to this vault"),
            Self::InvalidPublicKey => write!(f, "public key is invalid"),
            Self::UnknownEcdhKeyType => write!(f, "unknown ECDH key type"),
            Self::InvalidKeyType => write!(f, "invalid key type"),
            Self::EntryNotFound(entry) => write!(f, "{entry}"),
            Self::InvalidAesKeyLength => write!(f, "invalid AES key length"),
            Self::InvalidSecretLength(secret_type, actual, expected) => write!(
                f,
                "invalid secret length for {}. Actual: {}, Expected: {}",
                secret_type, actual, expected
            ),
            Self::InvalidHkdfOutputType => write!(f, "invalid HKDF outputtype"),
            Self::InvalidPrivateKeyLen => write!(f, "invalid private key length"),
            Self::AeadAesGcmEncrypt => write!(f, "aes encryption failed"),
            Self::AeadAesGcmDecrypt => write!(f, "aes decryption failed"),
            Self::HkdfExpandError => write!(f, "hkdf key expansion failed"),
            Self::SecretNotFound => write!(f, "secret not found"),
            Self::InvalidX25519SecretLength => write!(f, "invalid X25519 secret length"),
            Self::InvalidEd25519Secret => write!(f, "invalid Ed25519 secret"),
            Self::InvalidSecretAttributes => write!(f, "invalid secret attributes"),
            Self::StorageError => write!(f, "invalid storage"),
            Self::InvalidStorageData => write!(f, "invalid storage data"),
        }
    }
}

impl From<VaultError> for Error {
    #[track_caller]
    fn from(err: VaultError) -> Self {
        use VaultError::*;
        let kind = match err {
            SecretFromAnotherVault
            | InvalidPublicKey
            | InvalidKeyType
            | InvalidAesKeyLength
            | InvalidHkdfOutputType
            | InvalidPrivateKeyLen
            | InvalidX25519SecretLength => Kind::Misuse,
            UnknownEcdhKeyType | EntryNotFound(_) | SecretNotFound => Kind::NotFound,
            _ => Kind::Invalid,
        };

        Error::new(Origin::Vault, kind, err)
    }
}
