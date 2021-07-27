use ockam_core::Error;

/// Represents the failures that can occur in
/// an Ockam vault
#[derive(Clone, Copy, Debug)]
pub enum VaultError {
    /// No error
    None,
    /// Secret does not belong to this vault
    SecretFromAnotherVault,
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
    /// Invalid Curve25519 secret length
    InvalidCurve25519SecretLength,
    /// Invalid Curve25519 secret (scalar not clamped)
    InvalidCurve25519Secret,
    /// Invalid BLS secret length
    InvalidBlsSecretLength,
    /// Invalid BLS secret
    InvalidBlsSecret,
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
