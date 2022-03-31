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
use ockam_core::{async_trait, compat::boxed::Box, vault::Secret, Result};
use zeroize::Zeroize;

/// A trait implemented by both Initiator and Responder peers.
#[async_trait]
pub trait KeyExchanger {
    /// Return key exchange unique name.
    async fn name(&self) -> Result<String>;
    /// Generate request that should be sent to the other party.
    async fn generate_request(&mut self, payload: &[u8]) -> Result<Vec<u8>>;
    /// Handle response from other party and return payload.
    async fn handle_response(&mut self, response: &[u8]) -> Result<Vec<u8>>;
    /// Returns true if the key exchange process is complete.
    async fn is_complete(&self) -> Result<bool>;
    /// Return the data and keys needed for channels. Key exchange must be completed prior to calling this function.
    async fn finalize(self) -> Result<CompletedKeyExchange>;
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
#[derive(Debug, Zeroize)]
#[zeroize(drop)]
pub struct CompletedKeyExchange {
    h: [u8; 32],
    encrypt_key: Secret,
    decrypt_key: Secret,
}

impl CompletedKeyExchange {
    /// The state hash.
    pub fn h(&self) -> &[u8; 32] {
        &self.h
    }
    /// The derived encryption key.
    pub fn encrypt_key(&self) -> &Secret {
        &self.encrypt_key
    }
    /// The derived decryption key.
    pub fn decrypt_key(&self) -> &Secret {
        &self.decrypt_key
    }
}

impl CompletedKeyExchange {
    /// Build a CompletedKeyExchange comprised of the input parameters.
    pub fn new(h: [u8; 32], encrypt_key: Secret, decrypt_key: Secret) -> Self {
        CompletedKeyExchange {
            h,
            encrypt_key,
            decrypt_key,
        }
    }
}
