use crate::zdrop_impl;
use cfg_if::cfg_if;
use serde::{Deserialize, Serialize};
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
    if #[cfg(feature = "no_std")] {
        use heapless::consts::*;
        /// Secret Key Vector
        pub type SecretKeyVec = heapless::Vec<u8, U32>;
        /// Public Key Vector
        pub type PublicKeyVec = heapless::Vec<u8, U65>;
        /// Bufer for small vectors (e.g. array of attributes). Max size - 4
        pub type SmallBuffer<T> = heapless::Vec<T, U4>;
        /// Buffer for large binaries (e.g. encrypted data). Max size - 512
        pub type Buffer<T> = heapless::Vec<T, U512>;
        pub type KeyId = heapless::String<U64>;

        impl From<&str> for KeyId {
            fn from(s: &str) -> Self {
                heapless::String::from(s)
            }
        }
    }
    else {
        extern crate alloc;
        use alloc::vec::Vec;
        use alloc::string::String;
        /// Secret Key Vector
        pub type SecretKeyVec = Vec<u8>;
        /// Public Key Vector
        pub type PublicKeyVec = Vec<u8>;
        /// Buffer for small vectors (e.g. array of attributes)
        pub type SmallBuffer<T> = Vec<T>;
        /// Buffer for large binaries (e.g. encrypted data)
        pub type Buffer<T> = Vec<T>;
        /// ID of a Key
        pub type KeyId = String;
    }
}

/// Binary representation of a Secret.
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, Zeroize)]
pub struct SecretKey(SecretKeyVec);

impl SecretKey {
    /// Create a new secret key
    pub fn new(data: SecretKeyVec) -> Self {
        Self(data)
    }
}

impl AsRef<[u8]> for SecretKey {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// A public key
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, Zeroize)]
pub struct PublicKey(PublicKeyVec);

impl PublicKey {
    /// Create a new public key
    pub fn new(data: PublicKeyVec) -> Self {
        Self(data)
    }
}

impl AsRef<[u8]> for PublicKey {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// All possible [`SecretType`]s
#[derive(Serialize, Deserialize, Copy, Clone, Debug, Eq, PartialEq, Zeroize)]
pub enum SecretType {
    /// Secret buffer
    Buffer,
    /// AES key
    Aes,
    /// Curve 22519 key
    Curve25519,
    /// P256 key
    P256,
    /// BLS key
    Bls,
}

/// Possible [`SecretKey`]'s persistence
#[derive(Serialize, Deserialize, Copy, Clone, Debug, Eq, PartialEq, Zeroize)]
pub enum SecretPersistence {
    /// An ephemeral/temporary secret
    Ephemeral,
    /// A persistent secret
    Persistent,
}

/// Attributes for a specific vault [`SecretKey`]
#[derive(Serialize, Deserialize, Copy, Clone, Debug, Eq, PartialEq, Zeroize)]
pub struct SecretAttributes {
    stype: SecretType,
    persistence: SecretPersistence,
    length: usize,
}

impl SecretAttributes {
    /// Return the type of secret
    pub fn stype(&self) -> SecretType {
        self.stype
    }
    /// Return the persistence of the secret
    pub fn persistence(&self) -> SecretPersistence {
        self.persistence
    }
    /// Return the length of the secret
    pub fn length(&self) -> usize {
        self.length
    }
}

impl SecretAttributes {
    /// Create a new secret attribute
    pub fn new(stype: SecretType, persistence: SecretPersistence, length: usize) -> Self {
        SecretAttributes {
            stype,
            persistence,
            length,
        }
    }
}

zdrop_impl!(SecretKey);
