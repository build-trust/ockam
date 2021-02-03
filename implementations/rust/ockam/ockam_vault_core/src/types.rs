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
}

impl AsRef<[u8]> for SecretKey {
    fn as_ref(&self) -> &[u8] {
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
}

impl AsRef<[u8]> for PublicKey {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// The types of secret keys that the vault supports
#[derive(Copy, Clone, Debug, Eq, PartialEq, Zeroize)]
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

/// Persistence allowed by Secrets
#[derive(Copy, Clone, Debug, Eq, PartialEq, Zeroize)]
pub enum SecretPersistence {
    /// Secret is temporary
    Ephemeral,
    /// Secret is permanent
    Persistent,
}

/// Attributes for a specific vault secret
#[derive(Copy, Clone, Debug, Eq, PartialEq, Zeroize)]
pub struct SecretAttributes {
    /// The type of key
    pub stype: SecretType,
    /// How the key is persisted
    pub persistence: SecretPersistence,
    /// The purpose of the secret key
    pub length: usize,
}

zdrop_impl!(SecretKey);
