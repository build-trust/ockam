use failure::{Backtrace, Context, Fail};
use std::{fmt, io};

/// Represents the failures that can occur in
/// an Ockam Vault
#[derive(Clone, Copy, Fail, Debug)]
pub enum VaultFailErrorKind {
    /// Failed to initialize the vault
    #[fail(display = "Failed to initialize the vault")]
    Init,
    /// Failed to generate random bytes
    #[fail(display = "Failed to generate random bytes")]
    Random,
    /// Failed to compute SHA-256 digest
    #[fail(display = "Failed to compute SHA-256 digest")]
    Sha256,
    /// Failed to generate a secret
    #[fail(display = "Failed to generate a secret")]
    SecretGenerate,
    /// Failed to import a key
    #[fail(display = "Failed to import a key")]
    Import,
    /// Failed to export a key
    #[fail(display = "Failed to export a key")]
    Export,
    /// Failed to read attributes
    #[fail(display = "Failed to read attributes")]
    GetAttributes,
    /// Failed to find the specified public key
    #[fail(display = "Failed to find the specified public key")]
    PublicKey,
    /// Failed to compute elliptic curve diffie-hellman
    #[fail(display = "Failed to compute elliptic curve diffie-hellman")]
    Ecdh,
    /// Failed to compute HKDF SHA-256 digest
    #[fail(display = "Failed to compute HKDF SHA-256 digest")]
    HkdfSha256,
    /// Failed to encrypt data with AES-GCM
    #[fail(display = "Failed to encrypt data with AES-GCM")]
    AeadAesGcmEncrypt,
    /// Failed to decrypt data with AES-GCM
    #[fail(display = "Failed to decrypt data with AES-GCM")]
    AeadAesGcmDecrypt,
    /// Could not use the AES-GCM cipher scheme
    #[fail(display = "Could not use the AES-GCM cipher scheme")]
    AeadAesGcm,
    /// An invalid parameter was supplied: {}
    #[fail(display = "An invalid parameter was supplied: {}", 0)]
    InvalidParam(usize),
    /// Invalid attributes were specified
    #[fail(display = "Invalid attributes were specified")]
    InvalidAttributes,
    /// An invalid context was supplied
    #[fail(display = "An invalid context was supplied")]
    InvalidContext,
    /// An invalid buffer was supplied
    #[fail(display = "An invalid buffer was supplied")]
    InvalidBuffer,
    /// An invalid size was supplied
    #[fail(display = "An invalid size was supplied")]
    InvalidSize,
    /// An invalid key regeneration occurred
    #[fail(display = "An invalid key regeneration occurred")]
    InvalidRegenerate,
    /// An invalid secret was supplied
    #[fail(display = "An invalid secret was supplied")]
    InvalidSecret,
    /// Invalid secret attributes were supplied
    #[fail(display = "Invalid secret attributes were supplied")]
    InvalidSecretAttributes,
    /// An invalid secret type was supplied that is not supported
    #[fail(display = "An invalid secret type was supplied that is not supported")]
    InvalidSecretType,
    /// An invalid tag was supplied for decryption
    #[fail(display = "An invalid tag was supplied for decryption")]
    InvalidTag,
    /// The supplied buffer was too small
    #[fail(display = "The supplied buffer was too small")]
    BufferTooSmall,
    /// Default requires a specified random generator
    #[fail(display = "Default requires a specified random generator")]
    DefaultRandomRequired,
    /// Default requires a specified memory handler
    #[fail(display = "Default requires a specified memory handler")]
    MemoryRequired,
    /// The secret size specified does not match the expected value
    #[fail(display = "The secret size specified does not match the expected value")]
    SecretSizeMismatch,
    /// An error occurred while reading from I/O
    #[fail(display = "An error occurred while reading from I/O")]
    IOError,
    /// Unable to access the vault
    #[fail(display = "Access denied to the vault")]
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
    inner: Context<VaultFailErrorKind>,
}

impl VaultFailError {
    /// Convert from an error kind and a static string
    pub fn from_msg<D>(kind: VaultFailErrorKind, msg: D) -> Self
    where
        D: std::fmt::Display + std::fmt::Debug + Send + Sync + 'static,
    {
        Self {
            inner: Context::new(msg).context(kind),
        }
    }

    /// Convert to an integer, reused in From trait implementations
    fn to_usize(&self) -> usize {
        self.inner.get_context().to_usize()
    }
}

impl From<VaultFailErrorKind> for VaultFailError {
    fn from(kind: VaultFailErrorKind) -> Self {
        Self {
            inner: Context::new("").context(kind),
        }
    }
}

impl From<VaultFailError> for VaultFailErrorKind {
    fn from(err: VaultFailError) -> Self {
        *err.inner.get_context()
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

#[cfg(feature = "ffi")]
impl From<VaultFailErrorKind> for ffi_support::ExternError {
    fn from(err: VaultFailErrorKind) -> ffi_support::ExternError {
        ffi_support::ExternError::new_error(ffi_support::ErrorCode::new(err.to_usize() as i32), "")
    }
}

impl From<std::num::ParseIntError> for VaultFailErrorKind {
    fn from(_: std::num::ParseIntError) -> Self {
        VaultFailErrorKind::IOError
    }
}

impl From<io::Error> for VaultFailError {
    fn from(err: io::Error) -> Self {
        Self::from_msg(VaultFailErrorKind::IOError, format!("{:?}", err))
    }
}

impl From<Context<VaultFailErrorKind>> for VaultFailError {
    fn from(inner: Context<VaultFailErrorKind>) -> Self {
        Self { inner }
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

impl From<std::num::ParseIntError> for VaultFailError {
    fn from(_: std::num::ParseIntError) -> Self {
        VaultFailError::from(VaultFailErrorKind::IOError)
    }
}

#[cfg(all(target_os = "macos", feature = "os"))]
impl From<security_framework::base::Error> for VaultFailError {
    fn from(err: security_framework::base::Error) -> Self {
        match err.code() {
            -128 => VaultFailErrorKind::AccessDenied.into(),
            -25300 => VaultFailErrorKind::InvalidContext.into(),
            _ => VaultFailErrorKind::IOError.into(),
        }
    }
}

#[cfg(all(target_os = "macos", feature = "os"))]
impl From<keychain_services::Error> for VaultFailError {
    fn from(err: keychain_services::Error) -> Self {
        use keychain_services::ErrorKind::*;
        println!("ErrorKind = {:?}", err.kind());
        match err.kind() {
            AuthFailed => VaultFailErrorKind::AccessDenied.into(),
            BufferTooSmall => VaultFailErrorKind::BufferTooSmall.into(),
            CreateChainFailed => VaultFailErrorKind::IOError.into(),
            DataTooLarge => VaultFailErrorKind::InvalidBuffer.into(),
            DataNotAvailable => VaultFailErrorKind::InvalidContext.into(),
            DataNotModifiable => VaultFailErrorKind::IOError.into(),
            DuplicateCallback => VaultFailErrorKind::IOError.into(),
            DuplicateItem => VaultFailErrorKind::Import.into(),
            DuplicateKeychain => VaultFailErrorKind::IOError.into(),
            InDarkWake => VaultFailErrorKind::Init.into(),
            InteractionNotAllowed => VaultFailErrorKind::AccessDenied.into(),
            InteractionRequired => VaultFailErrorKind::AccessDenied.into(),
            InvalidCallback => VaultFailErrorKind::IOError.into(),
            InvalidItemRef => VaultFailErrorKind::InvalidContext.into(),
            InvalidKeychain => VaultFailErrorKind::IOError.into(),
            InvalidPrefsDomain => VaultFailErrorKind::IOError.into(),
            InvalidSearchRef => VaultFailErrorKind::InvalidContext.into(),
            ItemNotFound => VaultFailErrorKind::InvalidContext.into(),
            KeySizeNotAllowed => VaultFailErrorKind::SecretSizeMismatch.into(),
            MissingEntitlement => VaultFailErrorKind::AccessDenied.into(),
            NoCertificateModule => VaultFailErrorKind::AccessDenied.into(),
            NoDefaultKeychain => VaultFailErrorKind::AccessDenied.into(),
            NoPolicyModule => VaultFailErrorKind::AccessDenied.into(),
            NoStorageModule => VaultFailErrorKind::AccessDenied.into(),
            NoSuchAttr => VaultFailErrorKind::InvalidAttributes.into(),
            NoSuchClass => VaultFailErrorKind::InvalidAttributes.into(),
            NoSuchKeychain => VaultFailErrorKind::AccessDenied.into(),
            NotAvailable => VaultFailErrorKind::IOError.into(),
            ReadOnly => VaultFailErrorKind::AccessDenied.into(),
            ReadOnlyAttr => VaultFailErrorKind::InvalidAttributes.into(),
            WrongSecVersion => VaultFailErrorKind::InvalidAttributes.into(),
            Io { .. } => VaultFailErrorKind::IOError.into(),
            CFError { .. } => VaultFailErrorKind::IOError.into(),
            Errno { .. } => VaultFailErrorKind::IOError.into(),
            OSError { .. } => VaultFailErrorKind::IOError.into(),
        }
    }
}

#[cfg(feature = "ffi")]
impl From<VaultFailError> for ffi_support::ExternError {
    fn from(err: VaultFailError) -> ffi_support::ExternError {
        let err: VaultFailErrorKind = err.into();
        err.into()
    }
}

impl From<p256::ecdsa::Error> for VaultFailError {
    fn from(_: p256::ecdsa::Error) -> Self {
        VaultFailErrorKind::Ecdh.into()
    }
}

from_int_impl!(VaultFailError, u32);
from_int_impl!(VaultFailError, u64);
from_int_impl!(VaultFailError, u128);
from_int_impl!(VaultFailErrorKind, u32);
from_int_impl!(VaultFailErrorKind, u64);
from_int_impl!(VaultFailErrorKind, u128);

impl Fail for VaultFailError {
    fn cause(&self) -> Option<&dyn Fail> {
        self.inner.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.inner.backtrace()
    }
}

impl fmt::Display for VaultFailError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut first = true;

        for cause in Fail::iter_chain(&self.inner) {
            if first {
                first = false;
                writeln!(f, "Error: {}", cause)?;
            } else {
                writeln!(f, "Caused by: {}", cause)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn into_unsigned() {
        let errors = vec![
            (
                VaultFailErrorKind::Init,
                VaultFailErrorKind::ERROR_INTERFACE_VAULT | 1,
            ),
            (
                VaultFailErrorKind::Random,
                VaultFailErrorKind::ERROR_INTERFACE_VAULT | 2,
            ),
            (
                VaultFailErrorKind::Sha256,
                VaultFailErrorKind::ERROR_INTERFACE_VAULT | 3,
            ),
            (
                VaultFailErrorKind::SecretGenerate,
                VaultFailErrorKind::ERROR_INTERFACE_VAULT | 4,
            ),
            (
                VaultFailErrorKind::Import,
                VaultFailErrorKind::ERROR_INTERFACE_VAULT | 5,
            ),
            (
                VaultFailErrorKind::Export,
                VaultFailErrorKind::ERROR_INTERFACE_VAULT | 6,
            ),
            (
                VaultFailErrorKind::GetAttributes,
                VaultFailErrorKind::ERROR_INTERFACE_VAULT | 7,
            ),
            (
                VaultFailErrorKind::PublicKey,
                VaultFailErrorKind::ERROR_INTERFACE_VAULT | 8,
            ),
            (
                VaultFailErrorKind::Ecdh,
                VaultFailErrorKind::ERROR_INTERFACE_VAULT | 9,
            ),
            (
                VaultFailErrorKind::HkdfSha256,
                VaultFailErrorKind::ERROR_INTERFACE_VAULT | 10,
            ),
            (
                VaultFailErrorKind::AeadAesGcmEncrypt,
                VaultFailErrorKind::ERROR_INTERFACE_VAULT | 11,
            ),
            (
                VaultFailErrorKind::AeadAesGcmDecrypt,
                VaultFailErrorKind::ERROR_INTERFACE_VAULT | 12,
            ),
            (
                VaultFailErrorKind::AeadAesGcm,
                VaultFailErrorKind::ERROR_INTERFACE_VAULT | 13,
            ),
            (
                VaultFailErrorKind::InvalidParam(0),
                VaultFailErrorKind::ERROR_INTERFACE_VAULT | 20,
            ),
            (
                VaultFailErrorKind::InvalidAttributes,
                VaultFailErrorKind::ERROR_INTERFACE_VAULT | 21,
            ),
            (
                VaultFailErrorKind::InvalidContext,
                VaultFailErrorKind::ERROR_INTERFACE_VAULT | 22,
            ),
            (
                VaultFailErrorKind::InvalidBuffer,
                VaultFailErrorKind::ERROR_INTERFACE_VAULT | 23,
            ),
            (
                VaultFailErrorKind::InvalidSize,
                VaultFailErrorKind::ERROR_INTERFACE_VAULT | 24,
            ),
            (
                VaultFailErrorKind::InvalidRegenerate,
                VaultFailErrorKind::ERROR_INTERFACE_VAULT | 25,
            ),
            (
                VaultFailErrorKind::InvalidSecret,
                VaultFailErrorKind::ERROR_INTERFACE_VAULT | 26,
            ),
            (
                VaultFailErrorKind::InvalidSecretAttributes,
                VaultFailErrorKind::ERROR_INTERFACE_VAULT | 27,
            ),
            (
                VaultFailErrorKind::InvalidSecretType,
                VaultFailErrorKind::ERROR_INTERFACE_VAULT | 28,
            ),
            (
                VaultFailErrorKind::InvalidTag,
                VaultFailErrorKind::ERROR_INTERFACE_VAULT | 29,
            ),
            (
                VaultFailErrorKind::BufferTooSmall,
                VaultFailErrorKind::ERROR_INTERFACE_VAULT | 30,
            ),
            (
                VaultFailErrorKind::DefaultRandomRequired,
                VaultFailErrorKind::ERROR_INTERFACE_VAULT | 31,
            ),
            (
                VaultFailErrorKind::MemoryRequired,
                VaultFailErrorKind::ERROR_INTERFACE_VAULT | 32,
            ),
            (
                VaultFailErrorKind::SecretSizeMismatch,
                VaultFailErrorKind::ERROR_INTERFACE_VAULT | 33,
            ),
            (
                VaultFailErrorKind::IOError,
                VaultFailErrorKind::ERROR_INTERFACE_VAULT | 40,
            ),
        ];

        for err in &errors {
            assert_eq!(err.0.to_usize(), err.1);
            assert_eq!(Into::<u32>::into(err.0), err.1 as u32);
            assert_eq!(Into::<u64>::into(err.0), err.1 as u64);
            assert_eq!(Into::<u128>::into(err.0), err.1 as u128);

            let verr: VaultFailError = err.0.into();
            assert_eq!(verr.to_usize(), err.1);
            assert_eq!(Into::<u32>::into(verr), err.1 as u32);
            let verr: VaultFailError = err.0.into();
            assert_eq!(Into::<u64>::into(verr), err.1 as u64);
            let verr: VaultFailError = err.0.into();
            assert_eq!(Into::<u128>::into(verr), err.1 as u128);
        }
    }

    #[test]
    fn display() {
        let verr: VaultFailError = VaultFailErrorKind::Sha256.into();
        let verr_str = format!("{}", verr);
        assert_eq!(
            "Error: Failed to compute SHA-256 digest\nCaused by: \n",
            verr_str.as_str()
        );
    }
}
