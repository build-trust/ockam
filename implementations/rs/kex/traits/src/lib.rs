#![deny(
    missing_docs,
    missing_debug_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unconditional_recursion,
    unused_import_braces,
    unused_lifetimes,
    unused_qualifications,
    unused_extern_crates,
    unused_parens,
    while_true
)]
//! Handles key exchange using Noise for Ockam channels

use ockam_common::error::OckamResult;
use ockam_vault::types::PublicKey;
use ockam_vault::Secret;
use std::sync::Arc;

/// The maximum bytes that will be transmitted in a single message
pub const MAX_XX_TRANSMIT_SIZE: usize = 16384;
/// The number of bytes in a SHA256 digest
pub const SHA256_SIZE: usize = 32;
/// The number of bytes in AES128 key
pub const AES128_KEYSIZE: usize = 16;
/// The number of bytes in AES256 key
pub const AES256_KEYSIZE: usize = 32;
/// The number of bytes in AES-GCM tag
pub const AES_GCM_TAGSIZE: usize = 16;

/// A KeyExchange implements these methods
/// A KeyExchange implementation should wrap a vault instance
pub trait KeyExchange {
    /// Returns Noise protocol name
    fn get_protocol_name(&self) -> &'static [u8];
    /// Create a new `HandshakeState` starting with the prologue
    fn prologue(&mut self) -> OckamResult<()>;
    /// Perform the diffie-hellman computation
    fn dh(&mut self, secret_handle: &Box<dyn Secret>, public_key: &[u8]) -> OckamResult<()>;
    /// mix hash step in Noise protocol
    fn mix_hash<B: AsRef<[u8]>>(&mut self, data: B) -> OckamResult<()>;
    /// Encrypt and mix step in Noise protocol
    fn encrypt_and_mix_hash<B: AsRef<[u8]>>(&mut self, plaintext: B) -> OckamResult<Vec<u8>>;
    /// Decrypt and mix step in Noise protocol
    fn decrypt_and_mix_hash<B: AsRef<[u8]>>(&mut self, ciphertext: B) -> OckamResult<Vec<u8>>;
    /// Split step in Noise protocol
    fn split(&mut self) -> OckamResult<(Box<dyn Secret>, Box<dyn Secret>)>;
    /// Finish the key exchange and return computed data
    fn finalize(
        self,
        encrypt_key: Box<dyn Secret>,
        decrypt_key: Box<dyn Secret>,
    ) -> OckamResult<CompletedKeyExchange>;
}

/// Represents either the Initiator or the Responder
pub trait KeyExchanger {
    /// Handle the current step in the key exchange process
    fn process(&mut self, data: &[u8]) -> OckamResult<Vec<u8>>;
    /// Is the key exchange process completed yet
    fn is_complete(&self) -> bool;
    /// If completed, then return the data and keys needed for channels
    fn finalize(self: Box<Self>) -> OckamResult<CompletedKeyExchange>;
}

/// XX cipher suites
#[derive(Copy, Clone, Debug)]
pub enum CipherSuite {
    /// Curve25519 Aes256-GCM Sha256
    Curve25519AesGcmSha256,
    /// P256 Aes128-GCM Sha256
    P256Aes128GcmSha256,
}

/// Instantiate a stateful key exchange vault instance
pub trait NewKeyExchanger<E: KeyExchanger = Self, F: KeyExchanger = Self> {
    /// Create a new Key Exchanger with the initiator role
    fn initiator(&self, identity_key: Option<Arc<Box<dyn Secret>>>) -> E;
    /// Create a new Key Exchanger with the responder role
    fn responder(&self, identity_key: Option<Arc<Box<dyn Secret>>>) -> F;
}

/// A Completed Key Exchange elements
#[derive(Debug)]
pub struct CompletedKeyExchange {
    /// The state hash
    pub h: [u8; 32],
    /// The derived encryption key handle
    pub encrypt_key: Box<dyn Secret>,
    /// The derived decryption key handle
    pub decrypt_key: Box<dyn Secret>,
    /// The long term static key handle
    pub local_static_secret: Arc<Box<dyn Secret>>,
    /// The long term static public key from remote party
    pub remote_static_public_key: PublicKey,
}

/// Errors thrown by Key exchange
pub mod error;
