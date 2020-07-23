#![deny(
    missing_docs,
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

//! Implements the Ockam vault interface and provides
//! a C FFI version.
//!
//! Vault represents a location where cryptographic keys live such
//! as secure enclaves, TPMs, HSMs, Keyrings, files, memory, etc.

#![cfg_attr(feature = "nightly", feature(doc_cfg))]

#[macro_use]
extern crate arrayref;
#[cfg(feature = "ffi")]
#[macro_use]
extern crate ffi_support;
#[cfg(feature = "ffi")]
#[macro_use]
extern crate lazy_static;

use crate::error::VaultFailError;
use zeroize::Zeroize;

/// Internal macros
#[macro_use]
mod macros;
/// Represents the errors that occur within a vault
pub mod error;
#[cfg(feature = "ffi")]
/// The ffi functions, structs, and constants
pub mod ffi;
/// Software implementation of Vault. No persistence
/// all keys are stored, operations happen in memory
pub mod software;
/// The various enumerations of options
pub mod types;

use types::*;

/// Represents the methods available to a Vault
pub trait Vault: Zeroize {
    /// Generate random bytes and fill them into `data`
    fn random(&mut self, data: &mut [u8]) -> Result<(), VaultFailError>;
    /// Compute the SHA-256 digest given input `data`
    fn sha256<B: AsRef<[u8]>>(&self, data: B) -> Result<[u8; 32], VaultFailError>;
    /// Create a new secret key
    fn secret_generate(
        &mut self,
        attributes: SecretKeyAttributes,
    ) -> Result<SecretKeyContext, VaultFailError>;
    /// Import a secret key into the vault
    fn secret_import(
        &mut self,
        secret: &SecretKey,
        attributes: SecretKeyAttributes,
    ) -> Result<SecretKeyContext, VaultFailError>;
    /// Export a secret key from the vault
    fn secret_export(&self, context: SecretKeyContext) -> Result<SecretKey, VaultFailError>;
    /// Set the attributes for a secret key
    fn secret_attributes_get(
        &self,
        context: SecretKeyContext,
    ) -> Result<SecretKeyAttributes, VaultFailError>;
    /// Return the associated public key given the secret key
    fn secret_public_key_get(&self, context: SecretKeyContext)
        -> Result<PublicKey, VaultFailError>;
    /// Remove a secret key from the vault
    fn secret_destroy(&mut self, context: SecretKeyContext) -> Result<(), VaultFailError>;
    /// Compute Elliptic-Curve Diffie-Hellman using this secret key
    /// and the specified uncompressed public key
    fn ec_diffie_hellman(
        &mut self,
        context: SecretKeyContext,
        peer_public_key: PublicKey,
    ) -> Result<Vec<u8>, VaultFailError>;
    /// Compute the HKDF-SHA256 using the specified salt and input key material
    /// and return the output key material of the specified length
    fn hkdf_sha256<B: AsRef<[u8]>>(
        &self,
        salt: B,
        ikm: B,
        okm_len: usize,
    ) -> Result<Vec<u8>, VaultFailError>;
    /// Encrypt a payload using AES-GCM
    fn aead_aes_gcm_encrypt<B: AsRef<[u8]>>(
        &self,
        context: SecretKeyContext,
        plaintext: B,
        nonce: B,
        aad: B,
    ) -> Result<Vec<u8>, VaultFailError>;
    /// Decrypt a payload using AES-GCM
    fn aead_aes_gcm_decrypt<B: AsRef<[u8]>>(
        &self,
        context: SecretKeyContext,
        cipher_text: B,
        nonce: B,
        aad: B,
    ) -> Result<Vec<u8>, VaultFailError>;
    /// Close and release all resources in use by the vault
    fn deinit(&mut self);
}
