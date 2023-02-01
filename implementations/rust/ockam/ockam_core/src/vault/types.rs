use crate::{
    errcode::{Kind, Origin},
    Error,
};
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
        /// Private Key Vector. The maximum size is 32 bytes.
        pub type PrivateKeyVec = heapless::Vec<u8, 32>;
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
        /// Private Key Vector.
        pub type PrivateKeyVec = Vec<u8>;
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

/// Binary representation of a Private key.
#[derive(Serialize, Deserialize, Clone, Zeroize, Encode, Decode)]
#[zeroize(drop)]
#[cbor(transparent)]
pub struct PrivateKey(#[n(0)] PrivateKeyVec);

impl PrivateKey {
    /// Create a new private key.
    pub fn new(data: PrivateKeyVec) -> Self {
        Self(data)
    }
}

impl core::fmt::Debug for PrivateKey {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        f.pad("<private key omitted>")
    }
}

impl Eq for PrivateKey {}
impl PartialEq for PrivateKey {
    fn eq(&self, o: &Self) -> bool {
        subtle::ConstantTimeEq::ct_eq(&self.0[..], &o.0[..]).into()
    }
}

impl AsRef<[u8]> for PrivateKey {
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
    #[n(2)] stype: KeyType,
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
    /// Corresponding private key type.
    pub fn stype(&self) -> KeyType {
        self.stype
    }
}

impl PublicKey {
    /// Create a new public key.
    pub fn new(data: PublicKeyVec, stype: KeyType) -> Self {
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

/// All possible [`KeyType`]s
#[derive(Serialize, Deserialize, Copy, Clone, Debug, Encode, Decode, Eq, PartialEq, Zeroize)]
#[rustfmt::skip]
#[cbor(index_only)]
pub enum KeyType {
    /// Secret buffer
    #[n(1)] Buffer,
    /// AES key
    #[n(2)] Aes,
    /// Curve 22519 key
    #[n(3)] X25519,
    /// Ed 22519 key
    #[n(4)] Ed25519,
    /// NIST P-256 key
    #[n(5)] NistP256
}

/// All possible [`PrivateKey`] persistence types
#[derive(Serialize, Deserialize, Copy, Clone, Encode, Decode, Debug, Eq, PartialEq)]
#[rustfmt::skip]
#[cbor(index_only)]
pub enum KeyPersistence {
    /// An ephemeral/temporary key
    #[n(1)] Ephemeral,
    /// A persistent key
    #[n(2)] Persistent,
}

/// Attributes for a specific vault.
#[derive(Serialize, Deserialize, Copy, Encode, Decode, Clone, Debug, Eq, PartialEq)]
#[rustfmt::skip]
pub struct KeyAttributes {
    #[n(1)] stype: KeyType,
    #[n(2)] persistence: KeyPersistence,
    #[n(3)] length: u32,
}

impl KeyAttributes {
    /// Return the type of the key.
    pub fn stype(&self) -> KeyType {
        self.stype
    }
    /// Return the persistence of the key.
    pub fn persistence(&self) -> KeyPersistence {
        self.persistence
    }
    /// Return the length of the key.
    pub fn length(&self) -> u32 {
        self.length
    }
}

impl KeyAttributes {
    /// Create a new key attribute.
    pub fn new(stype: KeyType, persistence: KeyPersistence, length: u32) -> Self {
        KeyAttributes {
            stype,
            persistence,
            length,
        }
    }
}

impl fmt::Display for KeyAttributes {
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

/// A public key + the key id of its private key
#[derive(Clone, Debug, Zeroize)]
#[zeroize(drop)]
pub struct KeyPair {
    key_id: KeyId,
    public: PublicKey,
}

impl KeyPair {
    /// KeyId of the private key
    pub fn key_id(&self) -> &KeyId {
        &self.key_id
    }
    /// Public Key
    pub fn public(&self) -> &PublicKey {
        &self.public
    }
}

impl KeyPair {
    /// Create a new key pair
    pub fn new(key_id: KeyId, public_key: PublicKey) -> Self {
        Self {
            key_id,
            public: public_key,
        }
    }
}

/// Private key stored in a Vault Storage
#[derive(Debug, Eq, PartialEq, Clone)]
pub struct VaultEntry {
    key_attributes: KeyAttributes,
    key: Key,
}

/// A private key or reference.
#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize, Encode, Decode)]
#[rustfmt::skip]
pub enum Key {
    /// A private key.
    #[n(0)] Key(#[n(0)] PrivateKey),
    /// Reference to an unmanaged, external private key of AWS KMS.
    #[n(1)] Aws(#[n(1)] KeyId)
}

impl Key {
    /// Treat this private key as a key (not a KeyId) and pull it out.
    /// TODO: this code will be removed with the extraction of a proper AWS KMS vault implementation
    /// # Panics
    ///
    /// If the key entry does not hold a private key.
    pub fn cast_as_key(&self) -> &PrivateKey {
        self.try_as_key().expect("`Key` holds a key")
    }

    /// Treat this key as a private key and try to pull the key out.
    pub fn try_as_key(&self) -> Result<&PrivateKey, Error> {
        if let Key::Key(k) = self {
            Ok(k)
        } else {
            Err(Error::new(
                Origin::Other,
                Kind::Misuse,
                "`Key` does not hold a key",
            ))
        }
    }
}

impl VaultEntry {
    /// Private key Attributes
    pub fn key_attributes(&self) -> KeyAttributes {
        self.key_attributes
    }

    /// Get the key part of this vault entry.
    pub fn key(&self) -> &Key {
        &self.key
    }
}

impl VaultEntry {
    /// Create a new vault entry.
    pub fn new(key_attributes: KeyAttributes, key: Key) -> Self {
        VaultEntry {
            key_attributes,
            key,
        }
    }

    /// Create a new vault entry with a private key.
    pub fn new_key(key_attributes: KeyAttributes, key: PrivateKey) -> Self {
        VaultEntry {
            key_attributes,
            key: Key::Key(key),
        }
    }

    /// Create a new vault entry with an external private key from AWS KMS.
    pub fn new_aws(key_attributes: KeyAttributes, kid: KeyId) -> Self {
        VaultEntry {
            key_attributes,
            key: Key::Aws(kid),
        }
    }
}
