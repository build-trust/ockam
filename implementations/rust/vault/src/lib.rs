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
#[cfg(feature = "ffi")]
#[macro_use]
extern crate ffi_support;
#[cfg(any(feature = "ffi", feature = "nif"))]
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate downcast;
#[macro_use]
extern crate ockam_common;

use crate::error::VaultFailError;
use zeroize::Zeroize;

/// Internal macros
#[macro_use]
mod macros;
// #[cfg(feature = "atecc608a")]
/// C Vault implementations
// pub mod c;
/// Represents the errors that occur within a vault
pub mod error;
#[cfg(feature = "ffi")]
/// The ffi functions, structs, and constants
pub mod ffi;
/// Software vault where keys are persisted to the filesystem
/// if permanent
pub mod file;
/// Vault backed by the OSX Keychain and Secure-Enclave Processor
// #[cfg(all(target_os = "macos", feature = "os"))]
// pub mod osx;
/// Software implementation of Vault. No persistence
/// all keys are stored, operations happen in memory
pub mod software;
/// The various enumerations of options
pub mod types;

use failure::_core::fmt::Debug;
use types::*;

/// Secret
pub trait Secret: Debug + Sync + Send + 'static + downcast::Any {}

downcast!(dyn Secret);

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
    ) -> Result<Box<dyn Secret>, VaultFailError>;
    /// Import a secret key into the vault
    fn secret_import(
        &mut self,
        secret: &SecretKey,
        attributes: SecretKeyAttributes,
    ) -> Result<Box<dyn Secret>, VaultFailError>;
    /// Export a secret key from the vault
    fn secret_export(&mut self, context: &Box<dyn Secret>) -> Result<SecretKey, VaultFailError>;
    /// Get the attributes for a secret key
    fn secret_attributes_get(
        &mut self,
        context: &Box<dyn Secret>,
    ) -> Result<SecretKeyAttributes, VaultFailError>;
    /// Return the associated public key given the secret key
    fn secret_public_key_get(
        &mut self,
        context: &Box<dyn Secret>,
    ) -> Result<PublicKey, VaultFailError>;
    /// Remove a secret key from the vault
    fn secret_destroy(&mut self, context: Box<dyn Secret>) -> Result<(), VaultFailError>;
    /// Compute Elliptic-Curve Diffie-Hellman using this secret key
    /// and the specified uncompressed public key
    fn ec_diffie_hellman(
        &mut self,
        context: &Box<dyn Secret>,
        peer_public_key: PublicKey,
    ) -> Result<Box<dyn Secret>, VaultFailError>;
    /// Compute Elliptic-Curve Diffie-Hellman using this secret key
    /// and the specified uncompressed public key and return the HKDF-SHA256
    /// output using the DH value as the HKDF ikm
    fn ec_diffie_hellman_hkdf_sha256(
        &mut self,
        context: &Box<dyn Secret>,
        peer_public_key: PublicKey,
        salt: &Box<dyn Secret>,
        info: &[u8],
        output_attributes: Vec<SecretKeyAttributes>,
    ) -> Result<Vec<Box<dyn Secret>>, VaultFailError>;
    /// Compute the HKDF-SHA256 using the specified salt and input key material
    /// and return the output key material of the specified length
    fn hkdf_sha256(
        &mut self,
        salt: &Box<dyn Secret>,
        info: &[u8],
        ikm: Option<&Box<dyn Secret>>,
        output_attributes: Vec<SecretKeyAttributes>,
    ) -> Result<Vec<Box<dyn Secret>>, VaultFailError>;
    /// Encrypt a payload using AES-GCM
    fn aead_aes_gcm_encrypt<B: AsRef<[u8]>, C: AsRef<[u8]>, D: AsRef<[u8]>>(
        &mut self,
        context: &Box<dyn Secret>,
        plaintext: B,
        nonce: C,
        aad: D,
    ) -> Result<Vec<u8>, VaultFailError>;
    /// Decrypt a payload using AES-GCM
    fn aead_aes_gcm_decrypt<B: AsRef<[u8]>, C: AsRef<[u8]>, D: AsRef<[u8]>>(
        &mut self,
        context: &Box<dyn Secret>,
        cipher_text: B,
        nonce: C,
        aad: D,
    ) -> Result<Vec<u8>, VaultFailError>;
    /// Close and release all resources in use by the vault
    fn deinit(&mut self);
    /// Generate a signature
    fn sign<B: AsRef<[u8]>>(
        &mut self,
        secret_key: &Box<dyn Secret>,
        data: B,
    ) -> Result<[u8; 64], VaultFailError>;
    /// Verify a signature
    fn verify<B: AsRef<[u8]>>(
        &mut self,
        signature: [u8; 64],
        public_key: PublicKey,
        data: B,
    ) -> Result<(), VaultFailError>;
}

/// The `DynVault` trait is a modification of `Vault` trait suitable
/// for trait objects.
pub trait DynVault {
    /// Generate random bytes and fill them into `data`
    fn random(&mut self, data: &mut [u8]) -> Result<(), VaultFailError>;
    /// Compute the SHA-256 digest given input `data`
    fn sha256(&self, data: &[u8]) -> Result<[u8; 32], VaultFailError>;
    /// Create a new secret key
    fn secret_generate(
        &mut self,
        attributes: SecretKeyAttributes,
    ) -> Result<Box<dyn Secret>, VaultFailError>;
    /// Import a secret key into the vault
    fn secret_import(
        &mut self,
        secret: &SecretKey,
        attributes: SecretKeyAttributes,
    ) -> Result<Box<dyn Secret>, VaultFailError>;
    /// Export a secret key from the vault
    fn secret_export(&mut self, context: &Box<dyn Secret>) -> Result<SecretKey, VaultFailError>;
    /// Get the attributes for a secret key
    fn secret_attributes_get(
        &mut self,
        context: &Box<dyn Secret>,
    ) -> Result<SecretKeyAttributes, VaultFailError>;
    /// Return the associated public key given the secret key
    fn secret_public_key_get(
        &mut self,
        context: &Box<dyn Secret>,
    ) -> Result<PublicKey, VaultFailError>;
    /// Remove a secret key from the vault
    fn secret_destroy(&mut self, context: Box<dyn Secret>) -> Result<(), VaultFailError>;
    /// Compute Elliptic-Curve Diffie-Hellman using this secret key
    /// and the specified uncompressed public key
    fn ec_diffie_hellman(
        &mut self,
        context: &Box<dyn Secret>,
        peer_public_key: PublicKey,
    ) -> Result<Box<dyn Secret>, VaultFailError>;
    /// Compute Elliptic-Curve Diffie-Hellman using this secret key
    /// and the specified uncompressed public key and return the HKDF-SHA256
    /// output using the DH value as the HKDF ikm
    fn ec_diffie_hellman_hkdf_sha256(
        &mut self,
        context: &Box<dyn Secret>,
        peer_public_key: PublicKey,
        salt: &Box<dyn Secret>,
        info: &[u8],
        output_attributes: Vec<SecretKeyAttributes>,
    ) -> Result<Vec<Box<dyn Secret>>, VaultFailError>;
    /// Compute the HKDF-SHA256 using the specified salt and input key material
    /// and return the output key material of the specified length
    fn hkdf_sha256(
        &mut self,
        salt: &Box<dyn Secret>,
        info: &[u8],
        ikm: Option<&Box<dyn Secret>>,
        output_attributes: Vec<SecretKeyAttributes>,
    ) -> Result<Vec<Box<dyn Secret>>, VaultFailError>;
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
    /// Close and release all resources in use by the vault
    fn deinit(&mut self);
    /// Generate a signature
    fn sign(
        &mut self,
        secret_key: &Box<dyn Secret>,
        data: &[u8],
    ) -> Result<[u8; 64], VaultFailError>;
    /// Verify a signature
    fn verify(
        &mut self,
        signature: [u8; 64],
        public_key: PublicKey,
        data: &[u8],
    ) -> Result<(), VaultFailError>;
}

impl<D: Vault + Send + Sync + 'static> DynVault for D {
    fn random(&mut self, data: &mut [u8]) -> Result<(), VaultFailError> {
        Vault::random(self, data)
    }

    fn sha256(&self, data: &[u8]) -> Result<[u8; 32], VaultFailError> {
        Vault::sha256(self, data)
    }

    fn secret_generate(
        &mut self,
        attributes: SecretKeyAttributes,
    ) -> Result<Box<dyn Secret>, VaultFailError> {
        Vault::secret_generate(self, attributes)
    }

    fn secret_import(
        &mut self,
        secret: &SecretKey,
        attributes: SecretKeyAttributes,
    ) -> Result<Box<dyn Secret>, VaultFailError> {
        Vault::secret_import(self, secret, attributes)
    }

    fn secret_export(&mut self, context: &Box<dyn Secret>) -> Result<SecretKey, VaultFailError> {
        Vault::secret_export(self, context)
    }

    fn secret_attributes_get(
        &mut self,
        context: &Box<dyn Secret>,
    ) -> Result<SecretKeyAttributes, VaultFailError> {
        Vault::secret_attributes_get(self, context)
    }

    fn secret_public_key_get(
        &mut self,
        context: &Box<dyn Secret>,
    ) -> Result<PublicKey, VaultFailError> {
        Vault::secret_public_key_get(self, context)
    }

    fn secret_destroy(&mut self, context: Box<dyn Secret>) -> Result<(), VaultFailError> {
        Vault::secret_destroy(self, context)
    }

    fn ec_diffie_hellman(
        &mut self,
        context: &Box<dyn Secret>,
        peer_public_key: PublicKey,
    ) -> Result<Box<dyn Secret>, VaultFailError> {
        Vault::ec_diffie_hellman(self, context, peer_public_key)
    }

    fn ec_diffie_hellman_hkdf_sha256(
        &mut self,
        context: &Box<dyn Secret>,
        peer_public_key: PublicKey,
        salt: &Box<dyn Secret>,
        info: &[u8],
        output_attributes: Vec<SecretKeyAttributes>,
    ) -> Result<Vec<Box<dyn Secret>>, VaultFailError> {
        Vault::ec_diffie_hellman_hkdf_sha256(
            self,
            context,
            peer_public_key,
            salt,
            info,
            output_attributes,
        )
    }

    fn hkdf_sha256(
        &mut self,
        salt: &Box<dyn Secret>,
        info: &[u8],
        ikm: Option<&Box<dyn Secret>>,
        output_attributes: Vec<SecretKeyAttributes>,
    ) -> Result<Vec<Box<dyn Secret>>, VaultFailError> {
        Vault::hkdf_sha256(self, salt, info, ikm, output_attributes)
    }

    fn aead_aes_gcm_encrypt(
        &mut self,
        context: &Box<dyn Secret>,
        plaintext: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> Result<Vec<u8>, VaultFailError> {
        Vault::aead_aes_gcm_encrypt(self, context, plaintext, nonce, aad)
    }

    fn aead_aes_gcm_decrypt(
        &mut self,
        context: &Box<dyn Secret>,
        cipher_text: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> Result<Vec<u8>, VaultFailError> {
        Vault::aead_aes_gcm_decrypt(self, context, cipher_text, nonce, aad)
    }

    fn deinit(&mut self) {
        Vault::deinit(self)
    }

    fn sign(
        &mut self,
        secret_key: &Box<dyn Secret>,
        data: &[u8],
    ) -> Result<[u8; 64], VaultFailError> {
        Vault::sign(self, secret_key, data)
    }

    fn verify(
        &mut self,
        signature: [u8; 64],
        public_key: PublicKey,
        data: &[u8],
    ) -> Result<(), VaultFailError> {
        Vault::verify(self, signature, public_key, data)
    }
}
