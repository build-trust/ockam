//! Key exchange types and traits of the Ockam library.
//!
//! This crate contains the key exchange types of the Ockam library and is intended
//! for use by other crates that provide features and add-ons to the main
//! Ockam library.
//!
//! The main Ockam crate re-exports types defined in this crate.
#![deny(unsafe_code)]
#![warn(
    missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    unused_import_braces,
    unused_qualifications
)]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate core;

#[cfg(feature = "alloc")]
extern crate alloc;

use ockam_core::compat::{string::String, vec::Vec};
use ockam_core::vault::Buffer;
use ockam_core::Result;
use ockam_core::{async_trait, compat::boxed::Box};

/// A trait implemented by both Initiator and Responder peers.
#[async_trait]
pub trait Cipher: Send + 'static {
    /// Sets nonce for out-of-order decryption
    fn set_nonce(&mut self, nonce: u64);

    /// AEAD encryption
    async fn encrypt_with_ad(&mut self, ad: &[u8], plaintext: &[u8]) -> Result<Buffer<u8>>;

    /// AEAD decryption
    async fn decrypt_with_ad(&mut self, ad: &[u8], ciphertext: &[u8]) -> Result<Buffer<u8>>;

    /// Rotate key
    async fn rekey(&mut self) -> Result<()>;
}

/// A trait implemented by both Initiator and Responder peers.
#[async_trait]
pub trait KeyExchanger: Send + 'static {
    /// Cipher used for further encryption
    type Cipher: Cipher;
    /// Return key exchange unique name.
    async fn name(&self) -> Result<String>;
    /// Generate request that should be sent to the other party.
    async fn generate_request(&mut self, payload: &[u8]) -> Result<Vec<u8>>;
    /// Handle response from other party and return payload.
    async fn handle_response(&mut self, response: &[u8]) -> Result<Vec<u8>>;
    /// Returns true if the key exchange process is complete.
    async fn is_complete(&self) -> Result<bool>;
    /// Return the data and keys needed for channels. Key exchange must be completed prior to calling this function.
    async fn finalize(self) -> Result<CompletedKeyExchange<Self::Cipher>>;
}

/// A creator of both initiator and responder peers of a key exchange.
#[async_trait]
pub trait NewKeyExchanger {
    /// Initiator
    type Initiator: KeyExchanger + Send + Sync + 'static;
    /// Responder
    type Responder: KeyExchanger + Send + Sync + 'static;

    /// Create a new Key Exchanger with the initiator role
    async fn initiator(&self) -> Result<Self::Initiator>;
    /// Create a new Key Exchanger with the responder role
    async fn responder(&self) -> Result<Self::Responder>;
}

/// The state of a completed key exchange.
#[derive(Debug)]
pub struct CompletedKeyExchange<C: Cipher> {
    h: [u8; 32],
    encryption_cipher: C,
    decryption_cipher: C,
}

impl<C: Cipher> CompletedKeyExchange<C> {
    /// The state hash.
    pub fn h(&self) -> &[u8; 32] {
        &self.h
    }
    /// The encryption cipher.
    pub fn encryption_cipher(&mut self) -> &mut C {
        &mut self.encryption_cipher
    }
    /// The decryption cipher.
    pub fn decryption_cipher(&mut self) -> &mut C {
        &mut self.decryption_cipher
    }
}

impl<C: Cipher> CompletedKeyExchange<C> {
    /// Build a CompletedKeyExchange comprised of the input parameters.
    pub fn new(h: [u8; 32], encryption_cipher: C, decryption_cipher: C) -> Self {
        Self {
            h,
            encryption_cipher,
            decryption_cipher,
        }
    }
}
