use zeroize::Zeroize;

/// The types of secret keys that the vault supports
#[derive(Copy, Clone, Debug)]
pub enum SecretKeyType {
    /// Raw buffer of bytes
    Buffer(usize),
    /// AES-128 bit key
    Aes128,
    /// AES-256 bit key
    Aes256,
    /// x25519 secret key
    Curve25519,
    /// NIST P-256 (secp256r1, prime256v1) secret key
    P256,
}

impl SecretKeyType {
    /// Convert enum to a number
    pub fn to_usize(&self) -> usize {
        match *self {
            SecretKeyType::Buffer(..) => 0,
            SecretKeyType::Aes128 => 1,
            SecretKeyType::Aes256 => 2,
            SecretKeyType::Curve25519 => 3,
            SecretKeyType::P256 => 4,
        }
    }
}

from_int_impl!(SecretKeyType, i8);
from_int_impl!(SecretKeyType, i16);
from_int_impl!(SecretKeyType, i32);
from_int_impl!(SecretKeyType, i64);
from_int_impl!(SecretKeyType, i128);
from_int_impl!(SecretKeyType, u8);
from_int_impl!(SecretKeyType, u16);
from_int_impl!(SecretKeyType, u32);
from_int_impl!(SecretKeyType, u64);
from_int_impl!(SecretKeyType, u128);

/// Persistence allowed by Secrets
#[derive(Copy, Clone, Debug)]
pub enum SecretPersistenceType {
    /// Secret is temporary
    Ephemeral,
    /// Secret is permanent
    Persistent,
}

impl SecretPersistenceType {
    /// Convert enum to a number
    pub fn to_usize(&self) -> usize {
        match *self {
            SecretPersistenceType::Ephemeral => 0,
            SecretPersistenceType::Persistent => 1,
        }
    }
}

from_int_impl!(SecretPersistenceType, i8);
from_int_impl!(SecretPersistenceType, i16);
from_int_impl!(SecretPersistenceType, i32);
from_int_impl!(SecretPersistenceType, i64);
from_int_impl!(SecretPersistenceType, i128);
from_int_impl!(SecretPersistenceType, u8);
from_int_impl!(SecretPersistenceType, u16);
from_int_impl!(SecretPersistenceType, u32);
from_int_impl!(SecretPersistenceType, u64);
from_int_impl!(SecretPersistenceType, u128);

/// Secrets specific purpose
#[derive(Copy, Clone, Debug)]
pub enum SecretPurposeType {
    /// Key exchange
    KeyAgreement,
}

impl SecretPurposeType {
    /// Convert enum to a number
    pub fn to_usize(&self) -> usize {
        match *self {
            SecretPurposeType::KeyAgreement => 0,
        }
    }
}

from_int_impl!(SecretPurposeType, i8);
from_int_impl!(SecretPurposeType, i16);
from_int_impl!(SecretPurposeType, i32);
from_int_impl!(SecretPurposeType, i64);
from_int_impl!(SecretPurposeType, i128);
from_int_impl!(SecretPurposeType, u8);
from_int_impl!(SecretPurposeType, u16);
from_int_impl!(SecretPurposeType, u32);
from_int_impl!(SecretPurposeType, u64);
from_int_impl!(SecretPurposeType, u128);

/// Attributes for a specific vault secret
#[derive(Copy, Clone, Debug)]
pub struct SecretKeyAttributes {
    /// The type of key
    pub xtype: SecretKeyType,
    /// How the key is persisted
    pub persistence: SecretPersistenceType,
    /// The purpose of the secret key
    pub purpose: SecretPurposeType,
}

/// A context that uses secret keys e.g. TEE, HSM, SEP.
/// This list is not meant to be exhaustive, just the ones supported
/// Ockam vault.
///
/// Future options include:
///
/// Key is backed by Amazon's CloudHSM
/// AmazonCloudHsm,
/// Key is backed by Amazon's KMS
/// AmazonKMS,
/// Key is backed by Box's Keysafe
/// BoxKeysafe,
/// Key is backed HashiCorp KeyVault
/// HashiCorpVault,
/// Key is backed by an Intel SGX enclave
/// IntelSgx,
/// Key is backed by an AMD's PSP enclave
/// AmdPsp,
/// Key is backed by an ARM's Trustzone enclave
/// ArmTrustzone,
/// Key is backed a Apple's secure enclave
/// SecureEnclave,
#[derive(Clone, Copy, Debug)]
pub enum SecretKeyContext {
    /// Key is backed by RAM
    Memory,
    /// Key is backed by a file
    File,
    /// Key is backed Azure KeyVault
    AzureKeyVault,
    /// Key is backed an OS keyring like Windows Credential Vault, Gnome Keyring, KWallet, or
    /// Security Framework
    OsKeyRing,
}

/// Represents specific secrets employable by the vault
#[derive(Clone, Debug, Zeroize)]
#[zeroize(drop)]
pub enum SecretKey {
    /// Raw buffer of bytes
    Buffer(Vec<u8>),
    /// AES-128 bit key
    Aes128([u8; 16]),
    /// AES-256 bit key
    Aes256([u8; 32]),
    /// x25519 secret key
    Curve25519([u8; 32]),
    /// NIST P-256 (secp256r1, prime256v1) secret key
    P256([u8; 32]),
}

impl AsRef<[u8]> for SecretKey {
    fn as_ref(&self) -> &[u8] {
        match self {
            SecretKey::Buffer(a) => a.as_slice(),
            SecretKey::Aes128(a) => a.as_ref(),
            SecretKey::Aes256(a) => a.as_ref(),
            SecretKey::Curve25519(a) => a.as_ref(),
            SecretKey::P256(a) => a.as_ref(),
        }
    }
}
