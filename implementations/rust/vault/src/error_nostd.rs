/// A stripped down VaultFailErrorKind which lacks failure messages
#[derive(Clone, Copy, Debug)]
pub enum VaultFailErrorKind {
    /// Failed to initialize the vault
    Init,
    /// Failed to generate random bytes
    Random,
    /// Failed to compute SHA-256 digest
    Sha256,
    /// Failed to generate a secret
    SecretGenerate,
    /// Failed to import a key
    Import,
    /// Failed to export a key
    Export,
    /// Failed to read attributes
    GetAttributes,
    /// Failed to find the specified public key
    PublicKey,
    /// Failed to compute elliptic curve diffie-hellman
    Ecdh,
    /// Failed to compute HKDF SHA-256 digest
    HkdfSha256,
    /// Failed to encrypt data with AES-GCM
    AeadAesGcmEncrypt,
    /// Failed to decrypt data with AES-GCM
    AeadAesGcmDecrypt,
    /// Could not use the AES-GCM cipher scheme
    AeadAesGcm,
    /// An invalid parameter was supplied: {}
    InvalidParam(usize),
    /// Invalid attributes were specified
    InvalidAttributes,
    /// An invalid context was supplied
    InvalidContext,
    /// An invalid buffer was supplied
    InvalidBuffer,
    /// An invalid size was supplied
    InvalidSize,
    /// An invalid key regeneration occurred
    InvalidRegenerate,
    /// An invalid secret was supplied
    InvalidSecret,
    /// Invalid secret attributes were supplied
    InvalidSecretAttributes,
    /// An invalid secret type was supplied that is not supported
    InvalidSecretType,
    /// An invalid tag was supplied for decryption
    InvalidTag,
    /// The supplied buffer was too small
    BufferTooSmall,
    /// Default requires a specified random generator
    DefaultRandomRequired,
    /// Default requires a specified memory handler
    MemoryRequired,
    /// The secret size specified does not match the expected value
    SecretSizeMismatch,
    /// An error occurred while reading from I/O
    IOError,
    /// Unable to access the vault
    AccessDenied,
}

impl VaultFailErrorKind {
    pub(crate) const ERROR_INTERFACE_VAULT: usize = 3 << 24;
    /// Convert to an integer
    pub fn to_usize(&self) -> usize {
        match *self {
            VaultFailErrorKind::Init => Self::ERROR_INTERFACE_VAULT | 1,
            VaultFailErrorKind::Random => Self::ERROR_INTERFACE_VAULT | 2,
            VaultFailErrorKind::Sha256 => Self::ERROR_INTERFACE_VAULT | 3,
            VaultFailErrorKind::SecretGenerate => Self::ERROR_INTERFACE_VAULT | 4,
            VaultFailErrorKind::Import => Self::ERROR_INTERFACE_VAULT | 5,
            VaultFailErrorKind::Export => Self::ERROR_INTERFACE_VAULT | 6,
            VaultFailErrorKind::GetAttributes => Self::ERROR_INTERFACE_VAULT | 7,
            VaultFailErrorKind::PublicKey => Self::ERROR_INTERFACE_VAULT | 8,
            VaultFailErrorKind::Ecdh => Self::ERROR_INTERFACE_VAULT | 9,
            VaultFailErrorKind::HkdfSha256 => Self::ERROR_INTERFACE_VAULT | 10,
            VaultFailErrorKind::AeadAesGcmEncrypt => Self::ERROR_INTERFACE_VAULT | 11,
            VaultFailErrorKind::AeadAesGcmDecrypt => Self::ERROR_INTERFACE_VAULT | 12,
            VaultFailErrorKind::AeadAesGcm => Self::ERROR_INTERFACE_VAULT | 13,
            VaultFailErrorKind::InvalidParam(..) => Self::ERROR_INTERFACE_VAULT | 20,
            VaultFailErrorKind::InvalidAttributes => Self::ERROR_INTERFACE_VAULT | 21,
            VaultFailErrorKind::InvalidContext => Self::ERROR_INTERFACE_VAULT | 22,
            VaultFailErrorKind::InvalidBuffer => Self::ERROR_INTERFACE_VAULT | 23,
            VaultFailErrorKind::InvalidSize => Self::ERROR_INTERFACE_VAULT | 24,
            VaultFailErrorKind::InvalidRegenerate => Self::ERROR_INTERFACE_VAULT | 25,
            VaultFailErrorKind::InvalidSecret => Self::ERROR_INTERFACE_VAULT | 26,
            VaultFailErrorKind::InvalidSecretAttributes => Self::ERROR_INTERFACE_VAULT | 27,
            VaultFailErrorKind::InvalidSecretType => Self::ERROR_INTERFACE_VAULT | 28,
            VaultFailErrorKind::InvalidTag => Self::ERROR_INTERFACE_VAULT | 29,
            VaultFailErrorKind::BufferTooSmall => Self::ERROR_INTERFACE_VAULT | 30,
            VaultFailErrorKind::DefaultRandomRequired => Self::ERROR_INTERFACE_VAULT | 31,
            VaultFailErrorKind::MemoryRequired => Self::ERROR_INTERFACE_VAULT | 32,
            VaultFailErrorKind::SecretSizeMismatch => Self::ERROR_INTERFACE_VAULT | 33,
            VaultFailErrorKind::IOError => Self::ERROR_INTERFACE_VAULT | 40,
            VaultFailErrorKind::AccessDenied => Self::ERROR_INTERFACE_VAULT | 50,
        }
    }
}

/// Wraps an error kind with context and backtrace logic
#[derive(Debug)]
pub struct VaultFailError {
    inner: VaultFailErrorKind,
}

impl VaultFailError {
    /// Convert from an error kind and a static string

    pub fn from_msg<D>(kind: VaultFailErrorKind, _msg: D) -> Self
        where
            D: core::fmt::Display + core::fmt::Debug + Send + Sync + 'static,
    {
        Self {
            inner: kind,
        }
    }

    /// Convert to an integer, reused in From trait implementations
    fn to_usize(&self) -> usize {
        self.inner.to_usize()
    }
}

impl From<VaultFailErrorKind> for VaultFailError {
    fn from(kind: VaultFailErrorKind) -> Self {
        Self {
            inner: kind,
        }
    }
}

impl From<VaultFailError> for VaultFailErrorKind {
    fn from(err: VaultFailError) -> Self {
        err.inner
    }
}

impl From<hkdf::InvalidLength> for VaultFailErrorKind {
    fn from(_: hkdf::InvalidLength) -> Self {
        VaultFailErrorKind::HkdfSha256
    }
}

impl From<hkdf::InvalidPrkLength> for VaultFailErrorKind {
    fn from(_: hkdf::InvalidPrkLength) -> Self {
        VaultFailErrorKind::HkdfSha256
    }
}

impl From<aes_gcm::Error> for VaultFailErrorKind {
    fn from(_: aes_gcm::Error) -> Self {
        VaultFailErrorKind::AeadAesGcm
    }
}

impl From<hkdf::InvalidLength> for VaultFailError {
    fn from(_: hkdf::InvalidLength) -> Self {
        VaultFailError::from(VaultFailErrorKind::HkdfSha256)
    }
}

impl From<hkdf::InvalidPrkLength> for VaultFailError {
    fn from(_: hkdf::InvalidPrkLength) -> Self {
        VaultFailError::from(VaultFailErrorKind::HkdfSha256)
    }
}

impl From<aes_gcm::Error> for VaultFailError {
    fn from(_: aes_gcm::Error) -> Self {
        VaultFailError::from(VaultFailErrorKind::AeadAesGcm)
    }
}


from_int_impl!(VaultFailError, u32);
from_int_impl!(VaultFailError, u64);
from_int_impl!(VaultFailError, u128);
from_int_impl!(VaultFailErrorKind, u32);
from_int_impl!(VaultFailErrorKind, u64);
from_int_impl!(VaultFailErrorKind, u128);
