use ockam_core::{
    errcode::{Kind, Origin},
    Error,
};

/// Represents the failures that can occur in
/// an Ockam vault
#[derive(Clone, Copy, Debug)]
pub enum VaultError {
    /// Secret does not belong to this vault
    SecretFromAnotherVault = 1,
    /// Public key is invalid
    InvalidPublicKey,
    /// Unknown ECDH key type
    UnknownEcdhKeyType,
    /// Invalid key type
    InvalidKeyType,
    /// Entry not found
    EntryNotFound,
    /// Invalid AES key length
    InvalidAesKeyLength,
    /// Invalid HKDF outputtype
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
    /// Invalid BLS secret length
    InvalidBlsSecretLength,
    /// Invalid BLS secret
    InvalidBlsSecret,
    /// IO error when saving
    StorageError,
}

impl ockam_core::compat::error::Error for VaultError {}
impl core::fmt::Display for VaultError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::SecretFromAnotherVault => write!(f, "secret does not belong to this vault"),
            Self::InvalidPublicKey => write!(f, "public key is invalid"),
            Self::UnknownEcdhKeyType => write!(f, "unknown ECDH key type"),
            Self::InvalidKeyType => write!(f, "invalid key type"),
            Self::EntryNotFound => write!(f, "entry not found"),
            Self::InvalidAesKeyLength => write!(f, "invalid AES key length"),
            Self::InvalidHkdfOutputType => write!(f, "invalid HKDF outputtype"),
            Self::InvalidPrivateKeyLen => write!(f, "invalid private key length"),
            Self::AeadAesGcmEncrypt => write!(f, "aes encryption failed"),
            Self::AeadAesGcmDecrypt => write!(f, "aes decryption failed"),
            Self::HkdfExpandError => write!(f, "hkdf key expansion failed"),
            Self::SecretNotFound => write!(f, "secret not found"),
            Self::InvalidX25519SecretLength => write!(f, "invalid X25519 secret length"),
            Self::InvalidEd25519Secret => write!(f, "invalid Ed25519 secret"),
            Self::InvalidBlsSecretLength => write!(f, "invalid BLS secret length"),
            Self::InvalidBlsSecret => write!(f, "invalid BLS secret"),
            Self::StorageError => write!(f, "invalid storage"),
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
            UnknownEcdhKeyType | EntryNotFound | SecretNotFound => Kind::NotFound,
            _ => Kind::Invalid,
        };

        Error::new(Origin::Vault, kind, err)
    }
}
