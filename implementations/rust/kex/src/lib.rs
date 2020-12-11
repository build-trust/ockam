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

use ockam_vault::{error::VaultFailError, types::PublicKey, Secret};
use std::sync::Arc;

#[macro_use]
extern crate ockam_vault;

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
trait KeyExchange {
    /// Returns Noise protocol name
    fn get_protocol_name(&self) -> &'static [u8];

    /// Create a new `HandshakeState` starting with the prologue
    fn prologue(&mut self) -> Result<(), VaultFailError>;
    /// Perform the diffie-hellman computation
    fn dh(
        &mut self,
        secret_handle: &Box<dyn Secret>,
        public_key: &[u8],
    ) -> Result<(), VaultFailError>;
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
    fn split(&mut self) -> Result<(Box<dyn Secret>, Box<dyn Secret>), VaultFailError>;
    /// Finish the key exchange and return computed data
    fn finalize(
        self,
        encrypt_key: Box<dyn Secret>,
        decrypt_key: Box<dyn Secret>,
    ) -> Result<CompletedKeyExchange, VaultFailError>;
}

/// Represents either the Initiator or the Responder
pub trait KeyExchanger {
    /// Handle the current step in the key exchange process
    fn process(&mut self, data: &[u8]) -> Result<Vec<u8>, KexExchangeFailError>;
    /// Is the key exchange process completed yet
    fn is_complete(&self) -> bool;
    /// If completed, then return the data and keys needed for channels
    fn finalize(self: Box<Self>) -> Result<CompletedKeyExchange, VaultFailError>;
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
#[cfg(feature = "ffi")]
/// FFI module
pub mod ffi;
/// Implementation of Signal's X3DH
pub mod x3dh;
/// Implementation of Noise XX Pattern
pub mod xx;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::xx::XXNewKeyExchanger;
    use ockam_vault::SecretVault;
    use ockam_vault_software::DefaultVault;
    use std::sync::{Arc, Mutex};

    #[allow(non_snake_case)]
    #[test]
    fn full_flow__correct_credentials__keys_should_match() {
        let vault_initiator = Arc::new(Mutex::new(DefaultVault::default()));
        let vault_responder = Arc::new(Mutex::new(DefaultVault::default()));
        let key_exchanger = XXNewKeyExchanger::new(
            CipherSuite::P256Aes128GcmSha256,
            vault_initiator.clone(),
            vault_responder.clone(),
        );

        let mut initiator = key_exchanger.initiator(None);
        let mut responder = key_exchanger.responder(None);

        let m1 = initiator.process(&[]).unwrap();
        let _ = responder.process(&m1).unwrap();
        let m2 = responder.process(&[]).unwrap();
        let _ = initiator.process(&m2).unwrap();
        let m3 = initiator.process(&[]).unwrap();
        let _ = responder.process(&m3).unwrap();

        let initiator = Box::new(initiator);
        let initiator = initiator.finalize().unwrap();
        let responder = Box::new(responder);
        let responder = responder.finalize().unwrap();

        let mut vault_in = vault_initiator.lock().unwrap();
        let mut vault_re = vault_responder.lock().unwrap();

        assert_eq!(initiator.h, responder.h);

        let s1 = vault_in.secret_export(&initiator.encrypt_key).unwrap();
        let s2 = vault_re.secret_export(&responder.decrypt_key).unwrap();

        assert_eq!(s1, s2);

        let s1 = vault_in.secret_export(&initiator.decrypt_key).unwrap();
        let s2 = vault_re.secret_export(&responder.encrypt_key).unwrap();

        assert_eq!(s1, s2);
    }
}
