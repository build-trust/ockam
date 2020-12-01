use crate::error::Result as OckamResult;

cfg_if!{
    if #[cfg(feature = "heapless")] {
        use crate::heapless::consts::*;
        pub type SecretKey = heapless::Vec<u8, U32>;
        pub type PublicKey = heapless::Vec<u8, U65>;
        pub type SmallBuffer<T> = heapless::Vec<T, U4>;
        pub type Buffer<T> = heapless::Vec<T, U512>;
    }
    else {
        pub type SecretKey = Vec<u8>;
        pub type PublicKey = Vec<u8>;
        pub type SmallBuffer<T> = Vec<T>;
        pub type Buffer<T> = Vec<T>;
    }
}

/// Represents a secret handle or context that is
/// stored in a vault
pub trait Secret: std::fmt::Debug + Send + downcast::Any + Sync + 'static {}
downcast!(dyn Secret);

/// Persistence allowed by Secrets
#[derive(Copy, Clone, Debug, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub enum SecretPersistence {
    Ephemeral,
    Persistent
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

/// Attributes for a specific vault secret
#[derive(Copy, Clone, Debug, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub struct SecretAttributes {
    /// The type of key
    pub stype: SecretType,
    /// How the key is persisted
    pub persistence: SecretPersistence,
    /// The purpose of the secret key
    pub length: u32,
}

pub trait SecretVault {
    /// Create a new secret key
    fn secret_generate(
        &mut self,
        attributes: SecretAttributes,
    ) -> OckamResult<Box<dyn Secret>>;
    /// Import a secret key into the vault
    fn secret_import(
        &mut self,
        secret: &SecretKey,
        attributes: SecretAttributes,
    ) -> OckamResult<Box<dyn Secret>>;
    /// Return the associated public key given the secret key
    fn secret_public_key_get(
        &mut self,
        context: &Box<dyn Secret>,
    ) -> OckamResult<PublicKey>;
}

pub trait SigningVault {
    /// Generate a signature
    fn sign(
        &mut self,
        secret_key: &Box<dyn Secret>,
        data: &[u8],
    ) -> OckamResult<[u8; 64]>;
    /// Verify a signature
    fn verify(
        &mut self,
        signature: [u8; 64],
        public_key: PublicKey,
        data: &[u8],
    ) -> OckamResult<()>;
}

pub trait SymmetricVault {
    /// Compute the SHA-256 digest given input `data`
    fn sha256(&self, data: &[u8]) -> OckamResult<[u8; 32]>;
    /// Compute Elliptic-Curve Diffie-Hellman using this secret key
    /// and the specified uncompressed public key
    fn ec_diffie_hellman(
        &mut self,
        context: &Box<dyn Secret>,
        peer_public_key: PublicKey,
    ) -> OckamResult<Box<dyn Secret>>;
    /// Compute the HKDF-SHA256 using the specified salt and input key material
    /// and return the output key material of the specified length
    fn hkdf_sha256(
        &mut self,
        salt: &Box<dyn Secret>,
        info: &[u8],
        ikm: Option<&Box<dyn Secret>>,
        output_attributes: Vec<SecretAttributes>,
    ) -> OckamResult<SmallBuffer<Box<dyn Secret>>>;
    /// Encrypt a payload using AES-GCM
    fn aead_aes_gcm_encrypt(
        &mut self,
        context: &Box<dyn Secret>,
        plaintext: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> OckamResult<Buffer<u8>>;
    /// Decrypt a payload using AES-GCM
    fn aead_aes_gcm_decrypt(
        &mut self,
        context: &Box<dyn Secret>,
        cipher_text: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> OckamResult<Buffer<u8>>;
}