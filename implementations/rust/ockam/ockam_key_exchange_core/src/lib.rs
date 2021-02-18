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
    /// The state hash
    pub h: [u8; 32],
    /// The derived encryption key handle
    pub encrypt_key: Secret,
    /// The derived decryption key handle
    pub decrypt_key: Secret,
    /// The long term static key handle
    pub local_static_secret: Secret,
    /// The long term static public key from remote party
    pub remote_static_public_key: PublicKey,
}
