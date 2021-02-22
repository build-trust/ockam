#![deny(
    // missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unused_import_braces,
    unused_qualifications,
    warnings
)]

use ockam_vault_core::{PublicKey, Secret};
use zeroize::Zeroize;

/// Represents either the Initiator or the Responder
pub trait KeyExchanger {
    /// Handle the current step in the key exchange process
    fn process(&mut self, data: &[u8]) -> ockam_core::Result<Vec<u8>>;
    /// Is the key exchange process completed yet
    fn is_complete(&self) -> bool;
    /// If completed, then return the data and keys needed for channels
    fn finalize(self) -> ockam_core::Result<CompletedKeyExchange>;
}

/// Instantiate a stateful key exchange vault instance
pub trait NewKeyExchanger<E: KeyExchanger = Self, F: KeyExchanger = Self> {
    /// Create a new Key Exchanger with the initiator role
    fn initiator(&self) -> E;
    /// Create a new Key Exchanger with the responder role
    fn responder(&self) -> F;
}

/// A Completed Key Exchange elements
#[derive(Debug, Zeroize)]
pub struct CompletedKeyExchange {
    h: [u8; 32],
    encrypt_key: Secret,
    decrypt_key: Secret,
    local_static_secret: Secret,
    remote_static_public_key: PublicKey,
}

impl CompletedKeyExchange {
    /// The state hash
    pub fn h(&self) -> &[u8; 32] {
        &self.h
    }
    /// The derived encryption key handle
    pub fn encrypt_key(&self) -> &Secret {
        &self.encrypt_key
    }
    /// The derived decryption key handle
    pub fn decrypt_key(&self) -> &Secret {
        &self.decrypt_key
    }
    /// The long term static key handle
    pub fn local_static_secret(&self) -> &Secret {
        &self.local_static_secret
    }
    /// The long term static public key from remote party
    pub fn remote_static_public_key(&self) -> &PublicKey {
        &self.remote_static_public_key
    }
}

impl CompletedKeyExchange {
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
