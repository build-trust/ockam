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

#[macro_use]
extern crate arrayref;

use error::*;

use ockam_vault::{
    error::VaultFailError,
    types::{PublicKey, SecretKeyContext},
    DynVault,
};

use std::sync::{Arc, Mutex};

/// The maximum bytes that will be transmitted in a single message
pub const MAX_XX_TRANSMIT_SIZE: usize = 16384;
/// The number of bytes in a SHA256 digest
pub const SHA256_SIZE: usize = 32;
/// The number of bytes in AES128 key
pub const AES128_KEYSIZE: usize = 16;
/// The number of bytes in AES256 key
pub const AES256_KEYSIZE: usize = 32;

/// Handles storing the current values for `h`, `ck`, and if its a key or not
#[derive(Copy, Clone, Debug)]
struct SymmetricStateData {
    h: [u8; SHA256_SIZE],
    ck: [u8; SHA256_SIZE],
}

impl Default for SymmetricStateData {
    fn default() -> Self {
        Self {
            h: [0u8; SHA256_SIZE],
            ck: [0u8; SHA256_SIZE],
        }
    }
}

/// The state of the handshake for a Noise session
#[derive(Copy, Clone, Debug)]
struct HandshakeStateData {
    ephemeral_public_key: PublicKey,
    ephemeral_secret_handle: SecretKeyContext,
    static_public_key: PublicKey,
    static_secret_handle: SecretKeyContext,
    remote_ephemeral_public_key: Option<PublicKey>,
    remote_static_public_key: Option<PublicKey>,
}

/// A KeyExchange implements these methods
/// A KeyExchange implementation should wrap a vault instance
trait KeyExchange {
    /// The inner class wrapped
    const CSUITE: &'static [u8];

    /// Create a new `HandshakeState` starting with the prologue
    fn prologue(&mut self) -> Result<(), VaultFailError>;
    /// Perform the diffie-hellman computation
    fn dh(
        &mut self,
        secret_handle: SecretKeyContext,
        public_key: PublicKey,
    ) -> Result<Vec<u8>, VaultFailError>;
    /// mix key step in Noise protocol
    fn mix_key<B: AsRef<[u8]>>(&mut self, hash: B) -> Result<(), VaultFailError>;
    /// mix hash step in Noise protocol
    fn mix_hash<B: AsRef<[u8]>>(&mut self, data: B) -> Result<(), VaultFailError>;
    /// Encrypt and mix step in Noise protocol
    fn encrypt_and_mix_hash<B: AsRef<[u8]>>(
        &mut self,
        plaintext: B,
    ) -> Result<Vec<u8>, VaultFailError>;
    /// Decrypt and mix step in Noise protocol
    fn decrypt_and_mix_hash<B: AsRef<[u8]>>(
        &mut self,
        ciphertext: B,
    ) -> Result<Vec<u8>, VaultFailError>;
    /// Split step in Noise protocol
    fn split(&mut self) -> Result<Vec<u8>, VaultFailError>;
    /// Finish the key exchange and return computed data
    fn finalize<B: AsRef<[u8]>, C: AsRef<[u8]>>(
        &mut self,
        encrypt_ref: B,
        decrypt_ref: C,
    ) -> Result<CompletedKeyExchange, VaultFailError>;
}

/// Represents either the Initiator or the Responder
pub trait KeyExchanger {
    /// Handle the current step in the key exchange process
    fn process<B: AsRef<[u8]>>(&mut self, data: B) -> Result<Vec<u8>, KexExchangeFailError>;
    /// Is the key exchange process completed yet
    fn is_complete(&self) -> bool;
    /// If completed, then return the data and keys needed for channels
    fn finalize(&mut self) -> Result<CompletedKeyExchange, VaultFailError>;
}

/// Instantiate a stateful key exchange vault instance
pub trait NewKeyExchanger<E: KeyExchanger = Self, F: KeyExchanger = Self> {
    /// Create a new Key Exchanger with the initiator role
    fn initiator(v: Arc<Mutex<dyn DynVault + Send>>) -> E;
    /// Create a new Key Exchanger with the responder role
    fn responder(v: Arc<Mutex<dyn DynVault + Send>>) -> F;
}

/// A Completed Key Exchange elements
#[derive(Copy, Clone, Debug)]
pub struct CompletedKeyExchange {
    /// The state hash
    pub h: [u8; 32],
    /// The derived encryption key handle
    pub encrypt_key: SecretKeyContext,
    /// The derived decryption key handle
    pub decrypt_key: SecretKeyContext,
    /// The long term static key handle
    pub local_static_secret: SecretKeyContext,
    /// The long term static public key from remote party
    pub remote_static_public_key: PublicKey,
}

/// Errors thrown by Key exchange
pub mod error;
#[cfg(feature = "ffi")]
/// FFI module
pub mod ffi;
/// Implementation of Noise XX Pattern
pub mod xx;
