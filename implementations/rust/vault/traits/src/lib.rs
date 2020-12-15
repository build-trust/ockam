#![deny(
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
#![warn(missing_docs)]
//! Ockam Vaults encapsulate the various software and hardware secure enclaves
//! that store and execute cryptographic operations

//! Implements the Ockam vault interface and provides
//! a C FFI version.
//!
//! Vault represents a location where cryptographic keys live such
//! as secure enclaves, TPMs, HSMs, Keyrings, files, memory, etc.

#![cfg_attr(feature = "nightly", feature(doc_cfg))]

#[macro_use]
extern crate arrayref;
#[macro_use]
extern crate downcast;
#[macro_use]
extern crate cfg_if;
#[macro_use]
extern crate ockam_common;

pub use zeroize;

use zeroize::Zeroize;

/// Internal macros
#[macro_use]
mod macros;
/// Represents the errors that occur within a vault
pub mod error;
/// The various enumerations of options
pub mod types;

use ockam_common::error::OckamResult;
use std::fmt::Debug;
use types::*;

/// Secret
pub trait Secret: Debug + Sync + Send + 'static + downcast::Any + Zeroize {}

downcast!(dyn Secret);

/// Vault trait with secret management functionality
pub trait SecretVault: Zeroize {
    /// Create a new secret key
    fn secret_generate(&mut self, attributes: SecretAttributes) -> OckamResult<Box<dyn Secret>>;
    /// Import a secret key into the vault
    fn secret_import(
        &mut self,
        secret: &[u8],
        attributes: SecretAttributes,
    ) -> OckamResult<Box<dyn Secret>>;
    /// Export a secret key from the vault
    fn secret_export(&mut self, context: &Box<dyn Secret>) -> OckamResult<SecretKey>;
    /// Get the attributes for a secret key
    fn secret_attributes_get(&mut self, context: &Box<dyn Secret>)
        -> OckamResult<SecretAttributes>;
    /// Return the associated public key given the secret key
    fn secret_public_key_get(&mut self, context: &Box<dyn Secret>) -> OckamResult<PublicKey>;
    /// Remove a secret key from the vault
    fn secret_destroy(&mut self, context: Box<dyn Secret>) -> OckamResult<()>;
}

/// Trait with sign functionality
pub trait SignerVault: Zeroize {
    /// Generate a signature
    fn sign(&mut self, secret_key: &Box<dyn Secret>, data: &[u8]) -> OckamResult<[u8; 64]>;
}

/// Trait with verify functionality
pub trait VerifierVault: Zeroize {
    /// Verify a signature
    fn verify(&mut self, signature: &[u8; 64], public_key: &[u8], data: &[u8]) -> OckamResult<()>;
}

/// Trait with symmetric encryption
pub trait SymmetricVault: Zeroize {
    /// Encrypt a payload using AES-GCM
    fn aead_aes_gcm_encrypt(
        &mut self,
        context: &Box<dyn Secret>,
        plaintext: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> OckamResult<Vec<u8>>;
    /// Decrypt a payload using AES-GCM
    fn aead_aes_gcm_decrypt(
        &mut self,
        context: &Box<dyn Secret>,
        cipher_text: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> OckamResult<Vec<u8>>;
}

/// Vault with asymmetric encryption functionality
pub trait AsymmetricVault: Zeroize {
    /// Compute Elliptic-Curve Diffie-Hellman using this secret key
    /// and the specified uncompressed public key
    fn ec_diffie_hellman(
        &mut self,
        context: &Box<dyn Secret>,
        peer_public_key: &[u8],
    ) -> OckamResult<Box<dyn Secret>>;
}

/// Vault with hashing functionality
pub trait HashVault: Zeroize {
    /// Compute the SHA-256 digest given input `data`
    fn sha256(&self, data: &[u8]) -> OckamResult<[u8; 32]>;
    /// Compute the HKDF-SHA256 using the specified salt and input key material
    /// and return the output key material of the specified length
    fn hkdf_sha256(
        &mut self,
        salt: &Box<dyn Secret>,
        info: &[u8],
        ikm: Option<&Box<dyn Secret>>,
        output_attributes: Vec<SecretAttributes>,
    ) -> OckamResult<Vec<Box<dyn Secret>>>;
}

/// Trait for vault with persistence capabilities
pub trait PersistentVault: Zeroize {
    /// Returns some String id that can be then used to retrieve secret from storage
    fn get_persistence_id(&self, secret: &Box<dyn Secret>) -> OckamResult<String>;

    /// Returns persistent secret using id
    fn get_persistent_secret(&self, persistence_id: &str) -> OckamResult<Box<dyn Secret>>;
}
