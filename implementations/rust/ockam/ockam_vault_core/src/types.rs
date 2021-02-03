use crate::zdrop_impl;
use cfg_if::cfg_if;
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
    if #[cfg(feature = "no-std")] {
        use heapless::consts::*;
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
        extern crate alloc;
        use alloc::vec::Vec;
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

/// Binary representation of a [`Secret`]
#[derive(Clone, Debug, Eq, PartialEq, Zeroize)]
pub struct SecretKey(SecretKeyVec);

impl SecretKey {
    pub fn new(data: SecretKeyVec) -> Self {
        Self(data)
    }
}

impl AsRef<[u8]> for SecretKey {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Zeroize)]
pub struct PublicKey(PublicKeyVec);

impl PublicKey {
    pub fn new(data: PublicKeyVec) -> Self {
        Self(data)
    }
}

impl AsRef<[u8]> for PublicKey {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// All possible types of [`Secret`]s
#[derive(Copy, Clone, Debug, Eq, PartialEq, Zeroize)]
pub enum SecretType {
    Buffer,
    Aes,
    Curve25519,
    P256,
}

/// Possible [`Secret`]'s persistence
#[derive(Copy, Clone, Debug, Eq, PartialEq, Zeroize)]
pub enum SecretPersistence {
    Ephemeral,
    Persistent,
}

/// Attributes for a specific vault [`Secret`]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Zeroize)]
pub struct SecretAttributes {
    pub stype: SecretType,
    pub persistence: SecretPersistence,
    pub length: usize,
}

zdrop_impl!(SecretKey);
