use ockam_core::{
    errcode::{Kind, Origin},
    thiserror, Error,
};

/// Represents the failures that can occur in
/// an Ockam vault
#[derive(Clone, Copy, Debug, thiserror::Error)]
pub enum VaultError {
    /// Secret does not belong to this vault
    #[error("secret does not belong to this vault")]
    SecretFromAnotherVault = 1,
    /// Public key is invalid
    #[error("public key is invalid")]
    InvalidPublicKey,
    /// Unknown ECDH key type
    #[error("unknown ECDH key type")]
    UnknownEcdhKeyType,
    /// Invalid key type
    #[error("invalid key type")]
    InvalidKeyType,
    /// Entry not found
    #[error("entry not found")]
    EntryNotFound,
    /// Invalid AES key length
    #[error("invalid AES key length")]
    InvalidAesKeyLength,
    /// Invalid Secret length
    InvalidSecretLength,
    /// Invalid HKDF output type
    InvalidHkdfOutputType,
    /// Invalid private key length
    #[error("invalid private key length")]
    InvalidPrivateKeyLen,
    /// AES encryption failed
    #[error("aes encryption failed")]
    AeadAesGcmEncrypt,
    /// AES decryption failed
    #[error("aes decryption failed")]
    AeadAesGcmDecrypt,
    /// HKDF key expansion failed
    #[error("hkdf key expansion failed")]
    HkdfExpandError,
    /// Secret not found
    #[error("secret not found")]
    SecretNotFound,
    /// Invalid X25519 secret length
    #[error("invalid X25519 secret length")]
    InvalidX25519SecretLength,
    /// Invalid Ed25519 secret
    #[error("invalid Ed25519 secret")]
    InvalidEd25519Secret,
    /// Invalid BLS secret length
    #[error("invalid BLS secret length")]
    InvalidBlsSecretLength,
    /// Invalid BLS secret
    #[error("invalid BLS secret")]
    InvalidBlsSecret,
    /// IO error when saving
    #[error("invalid storage")]
    StorageError,
}

impl From<VaultError> for Error {
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
