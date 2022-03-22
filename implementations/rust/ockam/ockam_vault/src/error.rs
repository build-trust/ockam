use ockam_core::Error;

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
    /// Invalid Secret length
    InvalidSecretLength,
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
    /// Invalid BLS secret length
    InvalidBlsSecretLength,
    /// Invalid BLS secret
    InvalidBlsSecret,
    /// IO error when saving
    StorageError,
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
            ockam_core::compat::format!("{}::{:?}", module_path!(), err),
        )
    }
}
