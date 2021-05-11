//! Key exchange types and traits of the Ockam library.
//!
//! This crate contains the key exchange types of the Ockam library and is intended
//! for use by other crates that provide features and add-ons to the main
//! Ockam library.
//!
//! The main Ockam crate re-exports types defined in this crate.
#![deny(
    missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unused_import_braces,
    unused_qualifications,
    warnings
)]

use ockam_core::Result;
use ockam_vault_core::{PublicKey, Secret};
use zeroize::Zeroize;

/// A trait implemented by both Initiator and Responder peers.
pub trait KeyExchanger {
    /// Run the current phase of the key exchange process.
    fn process(&mut self, data: &[u8]) -> Result<Vec<u8>>;
    /// Returns true if the key exchange process is complete.
    fn is_complete(&self) -> bool;
    /// Return the data and keys needed for channels. Key exchange must be completed prior to calling this function.
    fn finalize(self) -> Result<CompletedKeyExchange>;
}

/// A creator of both initiator and responder peers of a key exchange.
pub trait NewKeyExchanger {
    /// Initiator
    type Initiator: KeyExchanger + Send + 'static;
    /// Responder
    type Responder: KeyExchanger + Send + 'static;

    /// Create a new Key Exchanger with the initiator role
    fn initiator(&self) -> Result<Self::Initiator>;
    /// Create a new Key Exchanger with the responder role
    fn responder(&self) -> Result<Self::Responder>;
}

/// The state of a completed key exchange.
#[derive(Debug, Zeroize)]
pub struct CompletedKeyExchange {
    h: [u8; 32],
    encrypt_key: Secret,
    decrypt_key: Secret,
    local_static_secret: Secret,
    remote_static_public_key: PublicKey,
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
    /// The long term static key.
    pub fn local_static_secret(&self) -> &Secret {
        &self.local_static_secret
    }
    /// Remote peer well known public key.
    pub fn remote_static_public_key(&self) -> &PublicKey {
        &self.remote_static_public_key
    }
}

impl CompletedKeyExchange {
    /// Build a CompletedKeyExchange comprised of the input parameters.
    pub fn new(
        h: [u8; 32],
        encrypt_key: Secret,
        decrypt_key: Secret,
        local_static_secret: Secret,
        remote_static_public_key: PublicKey,
    ) -> Self {
        CompletedKeyExchange {
            h,
            encrypt_key,
            decrypt_key,
            local_static_secret,
            remote_static_public_key,
        }
    }
}
