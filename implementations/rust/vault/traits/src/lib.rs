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

use crate::error::VaultFailError;
use zeroize::Zeroize;

/// Internal macros
#[macro_use]
mod macros;
/// Represents the errors that occur within a vault
pub mod error;
/// The various enumerations of options
pub mod types;

use std::fmt::Debug;
use types::*;

/// Secret
pub trait Secret: Debug + Sync + Send + 'static + downcast::Any + Zeroize {}

downcast!(dyn Secret);

/// Vault trait with secret management functionality
pub trait SecretVault: Zeroize {
    /// Create a new secret key
    fn secret_generate(
        &mut self,
        attributes: SecretAttributes,
    ) -> Result<Box<dyn Secret>, VaultFailError>;
    /// Import a secret key into the vault
    fn secret_import(
        &mut self,
        secret: &[u8],
        attributes: SecretAttributes,
    ) -> Result<Box<dyn Secret>, VaultFailError>;
    /// Export a secret key from the vault
    fn secret_export(&mut self, context: &Box<dyn Secret>) -> Result<SecretKey, VaultFailError>;
    /// Get the attributes for a secret key
    fn secret_attributes_get(
        &mut self,
        context: &Box<dyn Secret>,
    ) -> Result<SecretAttributes, VaultFailError>;
    /// Return the associated public key given the secret key
    fn secret_public_key_get(
        &mut self,
        context: &Box<dyn Secret>,
    ) -> Result<PublicKey, VaultFailError>;
    /// Remove a secret key from the vault
    fn secret_destroy(&mut self, context: Box<dyn Secret>) -> Result<(), VaultFailError>;
}

/// Trait with sign functionality
pub trait SignerVault: Zeroize {
    /// Generate a signature
    fn sign(
        &mut self,
        secret_key: &Box<dyn Secret>,
        data: &[u8],
    ) -> Result<[u8; 64], VaultFailError>;
}

/// Trait with verify functionality
pub trait VerifierVault: Zeroize {
    /// Verify a signature
    fn verify(
        &mut self,
        signature: &[u8; 64],
        public_key: &[u8],
        data: &[u8],
    ) -> Result<(), VaultFailError>;
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
    ) -> Result<Vec<u8>, VaultFailError>;
    /// Decrypt a payload using AES-GCM
    fn aead_aes_gcm_decrypt(
        &mut self,
        context: &Box<dyn Secret>,
        cipher_text: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> Result<Vec<u8>, VaultFailError>;
}

/// Vault with asymmetric encryption functionality
pub trait AsymmetricVault: Zeroize {
    /// Compute Elliptic-Curve Diffie-Hellman using this secret key
    /// and the specified uncompressed public key
    fn ec_diffie_hellman(
        &mut self,
        context: &Box<dyn Secret>,
        peer_public_key: &[u8],
    ) -> Result<Box<dyn Secret>, VaultFailError>;
}

/// Vault with hashing functionality
pub trait HashVault: Zeroize {
    /// Compute the SHA-256 digest given input `data`
    fn sha256(&self, data: &[u8]) -> Result<[u8; 32], VaultFailError>;
    /// Compute the HKDF-SHA256 using the specified salt and input key material
    /// and return the output key material of the specified length
    fn hkdf_sha256(
        &mut self,
        salt: &Box<dyn Secret>,
        info: &[u8],
        ikm: Option<&Box<dyn Secret>>,
        output_attributes: Vec<SecretAttributes>,
    ) -> Result<Vec<Box<dyn Secret>>, VaultFailError>;
}

/// Trait for vault with persistence capabilities
pub trait PersistentVault: Zeroize {
    /// Returns some String id that can be then used to retrieve secret from storage
    fn get_persistence_id(&self, secret: &Box<dyn Secret>) -> Result<String, VaultFailError>;

    /// Returns persistent secret using id
    fn get_persistent_secret(
        &self,
        persistence_id: &str,
    ) -> Result<Box<dyn Secret>, VaultFailError>;
}
