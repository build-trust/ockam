use cfg_if::cfg_if;
use core::fmt;
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

#[cfg(feature = "tag")]
use crate::TypeTag;

/// Curve25519 private key length.
pub const CURVE25519_SECRET_LENGTH_U32: u32 = 32;
/// Curve25519 private key length.
pub const CURVE25519_SECRET_LENGTH_USIZE: usize = 32;

/// Curve25519 public key length.
pub const CURVE25519_PUBLIC_LENGTH_U32: u32 = 32;
/// Curve25519 public key length.
pub const CURVE25519_PUBLIC_LENGTH_USIZE: usize = 32;

/// AES256 private key length.
pub const AES256_SECRET_LENGTH_U32: u32 = 32;
/// AES256 private key length.
pub const AES256_SECRET_LENGTH_USIZE: usize = 32;

/// AES128 private key length.
pub const AES128_SECRET_LENGTH_U32: u32 = 16;
/// AES128 private key length.
pub const AES128_SECRET_LENGTH_USIZE: usize = 16;

cfg_if! {
    if #[cfg(not(feature = "alloc"))] {
        /// Secret Key Vector. The maximum size is 32 bytes.
        pub type SecretKeyVec = heapless::Vec<u8, 32>;
        /// Public Key Vector. The maximum size is 65 bytes.
        pub type PublicKeyVec = heapless::Vec<u8, 65>;
        /// Buffer for small vectors (e.g. an array of attributes). The maximum length is 4 elements.
        pub type SmallBuffer<T> = heapless::Vec<T, 4>;
        /// Buffer for large binaries (e.g. encrypted data). The maximum length is 512 elements.
        pub type Buffer<T> = heapless::Vec<T, 512>;
        /// Signature Vector. The maximum length is 64 characters.
        pub type KeyId = heapless::String<64>;
        /// Signature Vector. The maximum size is 112 bytes.
        pub type SignatureVec = heapless::Vec<u8, 112>;

        impl From<&str> for KeyId {
            fn from(s: &str) -> Self {
                heapless::String::from(s)
            }
        }
    }
    else {
        use alloc::vec::Vec;
        use alloc::string::String;
        /// Secret Key Vector.
        pub type SecretKeyVec = Vec<u8>;
        /// Public Key Vector.
        pub type PublicKeyVec = Vec<u8>;
        /// Buffer for small vectors. (e.g. an array of attributes)
        pub type SmallBuffer<T> = Vec<T>;
        /// Buffer for large binaries. (e.g. encrypted data)
        pub type Buffer<T> = Vec<T>;
        /// ID of a Key.
        pub type KeyId = String;
        /// Signature Vector.
        pub type SignatureVec = Vec<u8>;
    }
}

/// Binary representation of a Secret.
#[derive(Serialize, Deserialize, Clone, Zeroize)]
#[zeroize(drop)]
pub struct SecretKey(SecretKeyVec);

impl SecretKey {
    /// Create a new secret key.
    pub fn new(data: SecretKeyVec) -> Self {
        Self(data)
    }
}

impl core::fmt::Debug for SecretKey {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        f.pad("<secret key omitted>")
    }
}

impl Eq for SecretKey {}
impl PartialEq for SecretKey {
    fn eq(&self, o: &Self) -> bool {
        subtle::ConstantTimeEq::ct_eq(&self.0[..], &o.0[..]).into()
    }
}

impl AsRef<[u8]> for SecretKey {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// A public key.
#[derive(Encode, Decode, Serialize, Deserialize, Clone, Debug, Zeroize)]
#[zeroize(drop)]
#[rustfmt::skip]
#[cbor(map)]
pub struct PublicKey {
    #[cfg(feature = "tag")]
    #[serde(skip)]
    #[n(0)] tag: TypeTag<8922437>,
    #[b(1)] data: PublicKeyVec,
    #[n(2)] stype: SecretType,
}

impl Eq for PublicKey {}
impl PartialEq for PublicKey {
    fn eq(&self, o: &Self) -> bool {
        let choice = subtle::ConstantTimeEq::ct_eq(&self.data[..], &o.data[..]);
        choice.into() && self.stype == o.stype
    }
}

impl PublicKey {
    /// Public Key data.
    pub fn data(&self) -> &[u8] {
        &self.data
    }
    /// Corresponding secret key type.
    pub fn stype(&self) -> SecretType {
        self.stype
    }
}

impl PublicKey {
    /// Create a new public key.
    pub fn new(data: PublicKeyVec, stype: SecretType) -> Self {
        PublicKey {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            data,
            stype,
        }
    }
}

impl fmt::Display for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} {}", self.stype(), hex::encode(self.data()))
    }
}

/// Binary representation of Signature.
#[derive(Serialize, Deserialize, Clone, Debug, Zeroize)]
pub struct Signature(SignatureVec);

impl Signature {
    /// Create a new signature.
    pub fn new(data: SignatureVec) -> Self {
        Self(data)
    }
}

impl AsRef<[u8]> for Signature {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl Eq for Signature {}

impl PartialEq for Signature {
    fn eq(&self, o: &Self) -> bool {
        subtle::ConstantTimeEq::ct_eq(&self.0[..], &o.0[..]).into()
    }
}

impl From<Signature> for SignatureVec {
    fn from(sig: Signature) -> Self {
        sig.0
    }
}

/// All possible [`SecretType`]s
#[derive(Serialize, Deserialize, Copy, Clone, Debug, Encode, Decode, Eq, PartialEq, Zeroize)]
#[rustfmt::skip]
#[cbor(index_only)]
pub enum SecretType {
    /// Secret buffer
    #[n(1)] Buffer,
    /// AES key
    #[n(2)] Aes,
    /// Curve 22519 key
    #[n(3)] X25519,
    /// Curve 22519 key
    #[n(4)] Ed25519,
    /// NIST P-256 key
    #[n(5)] NistP256
}

/// All possible [`SecretKey`] persistence types
#[derive(Serialize, Deserialize, Copy, Clone, Encode, Decode, Debug, Eq, PartialEq)]
#[rustfmt::skip]
#[cbor(index_only)]
pub enum SecretPersistence {
    /// An ephemeral/temporary secret
    #[n(1)] Ephemeral,
    /// A persistent secret
    #[n(2)] Persistent,
}

/// Attributes for a specific vault.
#[derive(Serialize, Deserialize, Copy, Encode, Decode, Clone, Debug, Eq, PartialEq)]
#[rustfmt::skip]
pub struct SecretAttributes {
    #[n(1)] stype: SecretType,
    #[n(2)] persistence: SecretPersistence,
    #[n(3)] length: u32,
}

impl SecretAttributes {
    /// Return the type of the secret.
    pub fn stype(&self) -> SecretType {
        self.stype
    }
    /// Return the persistence of the secret.
    pub fn persistence(&self) -> SecretPersistence {
        self.persistence
    }
    /// Return the length of the secret.
    pub fn length(&self) -> u32 {
        self.length
    }
}

impl SecretAttributes {
    /// Create a new secret attribute.
    pub fn new(stype: SecretType, persistence: SecretPersistence, length: u32) -> Self {
        SecretAttributes {
            stype,
            persistence,
            length,
        }
    }
}

impl fmt::Display for SecretAttributes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?}({:?}) len:{}",
            self.stype(),
            self.persistence(),
            self.length()
        )
    }
}

/// A public key
#[derive(Clone, Debug, Zeroize)]
#[zeroize(drop)]
pub struct KeyPair {
    secret: KeyId,
    public: PublicKey,
}

impl KeyPair {
    /// Secret key
    pub fn secret(&self) -> &KeyId {
        &self.secret
    }
    /// Public Key
    pub fn public(&self) -> &PublicKey {
        &self.public
    }
}

impl KeyPair {
    /// Create a new key pair
    pub fn new(secret: KeyId, public: PublicKey) -> Self {
        Self { secret, public }
    }
}

/// Secret stored in a Vault Storage
#[derive(Debug, Eq, PartialEq, Clone)]
pub struct VaultEntry {
    key_attributes: SecretAttributes,
    secret: Secret,
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub enum Secret {
    Key(SecretKey),
    Ref(KeyId)
}

impl Secret {
    pub fn cast_as_key(&self) -> &SecretKey {
        if let Secret::Key(k) = self {
            k
        } else {
            panic!("`Secret` does not hold a key")
        }
    }
}

impl VaultEntry {
    /// Secret's Attributes
    pub fn key_attributes(&self) -> SecretAttributes {
        self.key_attributes
    }

    pub fn secret(&self) -> &Secret {
        &self.secret
    }
}

impl VaultEntry {
    pub fn new(key_attributes: SecretAttributes, secret: Secret) -> Self {
        VaultEntry {
            key_attributes,
            secret
        }
    }

    pub fn new_key(key_attributes: SecretAttributes, key: SecretKey) -> Self {
        VaultEntry {
            key_attributes,
            secret: Secret::Key(key)
        }
    }

    pub fn new_ref(key_attributes: SecretAttributes, kid: KeyId) -> Self {
        VaultEntry {
            key_attributes,
            secret: Secret::Ref(kid)
        }
    }
}
