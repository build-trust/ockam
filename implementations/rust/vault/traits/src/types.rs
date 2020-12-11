use crate::error::{VaultFailError, VaultFailErrorKind};
use zeroize::Zeroize;

/// Curve25519 private key length
pub const CURVE25519_SECRET_LENGTH: usize = 32;
/// Curve25519 public key length
pub const CURVE25519_PUBLIC_LENGTH: usize = 32;
/// P256 private key length
pub const P256_SECRET_LENGTH: usize = 32;
/// P256 public key length
pub const P256_PUBLIC_LENGTH: usize = 65;
/// AES256 private key length
pub const AES256_SECRET_LENGTH: usize = 32;
/// AES128 private key length
pub const AES128_SECRET_LENGTH: usize = 16;

cfg_if! {
    if #[cfg(feature = "heapless")] {
        use crate::heapless::consts::*;
        /// Secret Key Vector
        pub type SecretKeyVec = heapless::Vec<u8, U32>;
        /// Public Key Vector
        pub type PublicKeyVec = heapless::Vec<u8, U65>;
        /// Bufer for small vectors (e.g. array of attributes). Max size - 4
        pub type SmallBuffer<T> = heapless::Vec<T, U4>;
        /// Buffer for large binaries (e.g. encrypted data). Max size - 512
        pub type Buffer<T> = heapless::Vec<T, U512>;
    }
    else {
        /// Secret Key Vector
        pub type SecretKeyVec = Vec<u8>;
        /// Public Key Vector
        pub type PublicKeyVec = Vec<u8>;
        /// Bufer for small vectors (e.g. array of attributes)
        pub type SmallBuffer<T> = Vec<T>;
        /// Buffer for large binaries (e.g. encrypted data)
        pub type Buffer<T> = Vec<T>;
    }
}

/// Secret Key
#[derive(Clone, Debug, Eq, PartialEq, Zeroize)]
pub struct SecretKey(SecretKeyVec);

impl SecretKey {
    /// Constructor
    pub fn new(data: SecretKeyVec) -> Self {
        Self(data)
    }

    /// Returns slice
    pub fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// Public key
#[derive(Clone, Debug, Eq, PartialEq, Zeroize)]
pub struct PublicKey(PublicKeyVec);

impl PublicKey {
    /// Constructor
    pub fn new(data: PublicKeyVec) -> Self {
        Self(data)
    }

    /// Returns slice
    pub fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// The types of secret keys that the vault supports
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub enum SecretType {
    /// Raw buffer of bytes
    Buffer,
    /// AES key
    Aes,
    /// x25519 secret key
    Curve25519,
    /// NIST P-256 (secp256r1, prime256v1) secret key
    P256,
}

impl SecretType {
    /// Convert enum to a number
    pub fn to_usize(&self) -> usize {
        match *self {
            SecretType::Buffer => 0,
            SecretType::Aes => 1,
            SecretType::Curve25519 => 2,
            SecretType::P256 => 3,
        }
    }

    /// Try to convert from a number to the rust enum
    pub fn from_usize(value: usize) -> Result<Self, VaultFailError> {
        match value {
            0 => Ok(SecretType::Buffer),
            1 => Ok(SecretType::Aes),
            2 => Ok(SecretType::Curve25519),
            3 => Ok(SecretType::P256),
            _ => Err(VaultFailErrorKind::InvalidParam(0).into()),
        }
    }
}

from_int_impl!(SecretType, i8);
from_int_impl!(SecretType, i16);
from_int_impl!(SecretType, i32);
from_int_impl!(SecretType, i64);
from_int_impl!(SecretType, i128);
from_int_impl!(SecretType, u8);
from_int_impl!(SecretType, u16);
from_int_impl!(SecretType, u32);
from_int_impl!(SecretType, u64);
from_int_impl!(SecretType, u128);
try_from_int_impl!(SecretType, i8);
try_from_int_impl!(SecretType, i16);
try_from_int_impl!(SecretType, i32);
try_from_int_impl!(SecretType, i64);
try_from_int_impl!(SecretType, i128);
try_from_int_impl!(SecretType, u8);
try_from_int_impl!(SecretType, u16);
try_from_int_impl!(SecretType, u32);
try_from_int_impl!(SecretType, u64);
try_from_int_impl!(SecretType, u128);

/// Persistence allowed by Secrets
#[derive(Copy, Clone, Debug, Hash, Ord, PartialOrd, Eq, PartialEq, Zeroize)]
pub enum SecretPersistence {
    /// Secret is temporary
    Ephemeral,
    /// Secret is permanent
    Persistent,
}

impl SecretPersistence {
    /// Convert enum to a number
    pub fn to_usize(&self) -> usize {
        match *self {
            SecretPersistence::Ephemeral => 0,
            SecretPersistence::Persistent => 1,
        }
    }

    /// Try to convert from a number to the rust enum
    pub fn from_usize(value: usize) -> Result<Self, VaultFailError> {
        match value {
            0 => Ok(SecretPersistence::Ephemeral),
            1 => Ok(SecretPersistence::Persistent),
            _ => Err(VaultFailErrorKind::InvalidParam(0).into()),
        }
    }
}

from_int_impl!(SecretPersistence, i8);
from_int_impl!(SecretPersistence, i16);
from_int_impl!(SecretPersistence, i32);
from_int_impl!(SecretPersistence, i64);
from_int_impl!(SecretPersistence, i128);
from_int_impl!(SecretPersistence, u8);
from_int_impl!(SecretPersistence, u16);
from_int_impl!(SecretPersistence, u32);
from_int_impl!(SecretPersistence, u64);
from_int_impl!(SecretPersistence, u128);
try_from_int_impl!(SecretPersistence, i8);
try_from_int_impl!(SecretPersistence, i16);
try_from_int_impl!(SecretPersistence, i32);
try_from_int_impl!(SecretPersistence, i64);
try_from_int_impl!(SecretPersistence, i128);
try_from_int_impl!(SecretPersistence, u8);
try_from_int_impl!(SecretPersistence, u16);
try_from_int_impl!(SecretPersistence, u32);
try_from_int_impl!(SecretPersistence, u64);
try_from_int_impl!(SecretPersistence, u128);

/// Attributes for a specific vault secret
#[derive(Copy, Clone, Debug, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub struct SecretAttributes {
    /// The type of key
    pub stype: SecretType,
    /// How the key is persisted
    pub persistence: SecretPersistence,
    /// The purpose of the secret key
    pub length: usize,
}

impl SecretAttributes {
    /// Convert attributes to byte values
    pub fn to_bytes(&self) -> [u8; 6] {
        let mut output = [0u8; 6];
        output[..2].copy_from_slice((self.stype.to_usize() as u16).to_be_bytes().as_ref());
        output[2..4].copy_from_slice((self.persistence.to_usize() as u16).to_be_bytes().as_ref());
        output[4..].copy_from_slice((self.length as u16).to_be_bytes().as_ref());
        output
    }
}

impl std::convert::TryFrom<[u8; 6]> for SecretAttributes {
    type Error = VaultFailError;

    fn try_from(bytes: [u8; 6]) -> Result<Self, Self::Error> {
        let xtype = SecretType::from_usize(u16::from_be_bytes(*array_ref![bytes, 0, 2]) as usize)?;
        let persistence =
            SecretPersistence::from_usize(u16::from_be_bytes(*array_ref![bytes, 2, 2]) as usize)?;
        let len = u16::from_be_bytes(*array_ref![bytes, 4, 2]) as usize;
        Ok(Self {
            stype: xtype,
            persistence,
            length: len,
        })
    }
}

zdrop_impl!(SecretKey);
