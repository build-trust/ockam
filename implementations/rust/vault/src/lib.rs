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
//! Ockam Vaults encapsulate the various software and hardware secure enclaves
//! that store and execute cryptographic operations
//!
//!
//!

/*
 * If the feature is selected, the library will be set as no_std
 *     $ cargo build --features="nostd-stm32f4"
 *   or in a Cargo.toml dependency line
 *     [dependencies]
 *     ockam-vault = { version="x.y", features = ["nostd-stm32f4"] }
 * The "-tgt00" string is a placeholder to target
 * different implmentations that would
 * support of various families hardware parts with
 * features supporting the functionality of the vault.
 */
#![cfg_attr(feature = "nostd-stm32f4", no_std)]
// #![cfg_attr(feature = "nostd-stm32f4", feature(alloc_error_handler))]
#![feature(alloc_error_handler)]

//! Implements the Ockam vault interface and provides
//! a C FFI version.
//!
//! Vault represents a location where cryptographic keys live such
//! as secure enclaves, TPMs, HSMs, Keyrings, files, memory, etc.

#![cfg_attr(feature = "nightly", feature(doc_cfg))]


// #[cfg(not(feature = "nostd-stm32f4")]
// extern crate core;

/*
 * Though no_std is a desired option, the use of the alloc
 * crate or other support for Vec or other containers
 * may be a different decision per target hence the
 * '-tgt00' portion of the feature tag to allow differentiation
 */
#[cfg(feature = "nostd-stm32f4")]
#[macro_use]
extern crate alloc;

#[cfg(feature = "nostd-stm32f4")]
use alloc::vec::Vec;

#[cfg(feature = "nostd-stm32f4")]
#[allow(unused_imports)]
use panic_halt as _;

#[cfg(feature = "nostd-stm32f4")]
#[alloc_error_handler]
fn nostd_error_handler(layout: core::alloc::Layout) -> ! {
    panic!("memory allocation of {} bytes failed", layout.size())
}

#[cfg(feature = "nostd-stm32f4")]
use alloc_cortex_m::CortexMHeap;

#[cfg(feature = "nostd-stm32f4")]
#[global_allocator]
/// Provides a global_allocator needed in this crate (TODO fix?)
/// the allocator needs to be initialized at boot time
/// /* these symbols should come from the link script */
/// extern "C" {
///     static mut _heap_start: u32;
///     static mut _heap_end:   u32;
/// }
/// 
/// fn init_heap()
/// {
///     use ockam_vault::ALLOCATOR;
///
///     let start = unsafe { &mut _heap_start as *mut u32 as usize };
///     let end   = unsafe { &mut _heap_end   as *mut u32 as usize };
///     unsafe { ALLOCATOR.init(start, end - start) }
///  }
/// where init_heap needs to be called near the boot time configuration
/// of the device, before any allocations are used, and where
/// a link script sets up the values in the _heap_start, _heap_end symbols
/// and an even earlier stage
pub static ALLOCATOR: CortexMHeap = CortexMHeap::empty();

#[macro_use]
extern crate arrayref;
#[cfg(feature = "ffi")]
#[macro_use]
extern crate ffi_support;
#[cfg(any(feature = "ffi", feature = "nif"))]
#[macro_use]
extern crate lazy_static;
// #[macro_use]
// extern crate ockam_common;

use crate::error::VaultFailError;
use zeroize::Zeroize;

/// Internal macros
#[macro_use]
mod macros;
#[cfg(feature = "atecc608a")]
/// C Vault implementations
pub mod c;
/// Represents the errors that occur within a vault
pub mod error;
#[cfg(feature = "ffi")]
/// The ffi functions, structs, and constants
pub mod ffi;
/// Vault backed by the OSX Keychain and Secure-Enclave Processor
#[cfg(all(target_os = "macos", feature = "os"))]
pub mod osx;
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
    fn secret_export(&mut self, context: SecretKeyContext) -> Result<SecretKey, VaultFailError>;
    /// Get the attributes for a secret key
    fn secret_attributes_get(
        &mut self,
        context: SecretKeyContext,
    ) -> Result<SecretKeyAttributes, VaultFailError>;
    /// Return the associated public key given the secret key
    fn secret_public_key_get(
        &mut self,
        context: SecretKeyContext,
    ) -> Result<PublicKey, VaultFailError>;
    /// Remove a secret key from the vault
    fn secret_destroy(&mut self, context: SecretKeyContext) -> Result<(), VaultFailError>;
    /// Compute Elliptic-Curve Diffie-Hellman using this secret key
    /// and the specified uncompressed public key
    fn ec_diffie_hellman(
        &mut self,
        context: SecretKeyContext,
        peer_public_key: PublicKey,
    ) -> Result<SecretKeyContext, VaultFailError>;
    /// Compute Elliptic-Curve Diffie-Hellman using this secret key
    /// and the specified uncompressed public key and return the HKDF-SHA256
    /// output using the DH value as the HKDF ikm
    fn ec_diffie_hellman_hkdf_sha256(
        &mut self,
        context: SecretKeyContext,
        peer_public_key: PublicKey,
        salt: SecretKeyContext,
        info: &[u8],
        output_attributes: Vec<SecretKeyAttributes>,
    ) -> Result<Vec<SecretKeyContext>, VaultFailError>;
    /// Compute the HKDF-SHA256 using the specified salt and input key material
    /// and return the output key material of the specified length
    fn hkdf_sha256(
        &mut self,
        salt: SecretKeyContext,
        info: &[u8],
        ikm: Option<SecretKeyContext>,
        output_attributes: Vec<SecretKeyAttributes>,
    ) -> Result<Vec<SecretKeyContext>, VaultFailError>;
    /// Encrypt a payload using AES-GCM
    fn aead_aes_gcm_encrypt<B: AsRef<[u8]>, C: AsRef<[u8]>, D: AsRef<[u8]>>(
        &mut self,
        context: SecretKeyContext,
        plaintext: B,
        nonce: C,
        aad: D,
    ) -> Result<Vec<u8>, VaultFailError>;
    /// Decrypt a payload using AES-GCM
    fn aead_aes_gcm_decrypt<B: AsRef<[u8]>, C: AsRef<[u8]>, D: AsRef<[u8]>>(
        &mut self,
        context: SecretKeyContext,
        cipher_text: B,
        nonce: C,
        aad: D,
    ) -> Result<Vec<u8>, VaultFailError>;
    /// Close and release all resources in use by the vault
    fn deinit(&mut self);
    /// Generate a signature
    fn sign<B: AsRef<[u8]>>(
        &mut self,
        secret_key: SecretKeyContext,
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
    ) -> Result<SecretKeyContext, VaultFailError>;
    /// Import a secret key into the vault
    fn secret_import(
        &mut self,
        secret: &SecretKey,
        attributes: SecretKeyAttributes,
    ) -> Result<SecretKeyContext, VaultFailError>;
    /// Export a secret key from the vault
    fn secret_export(&mut self, context: SecretKeyContext) -> Result<SecretKey, VaultFailError>;
    /// Get the attributes for a secret key
    fn secret_attributes_get(
        &mut self,
        context: SecretKeyContext,
    ) -> Result<SecretKeyAttributes, VaultFailError>;
    /// Return the associated public key given the secret key
    fn secret_public_key_get(
        &mut self,
        context: SecretKeyContext,
    ) -> Result<PublicKey, VaultFailError>;
    /// Remove a secret key from the vault
    fn secret_destroy(&mut self, context: SecretKeyContext) -> Result<(), VaultFailError>;
    /// Compute Elliptic-Curve Diffie-Hellman using this secret key
    /// and the specified uncompressed public key
    fn ec_diffie_hellman(
        &mut self,
        context: SecretKeyContext,
        peer_public_key: PublicKey,
    ) -> Result<SecretKeyContext, VaultFailError>;
    /// Compute Elliptic-Curve Diffie-Hellman using this secret key
    /// and the specified uncompressed public key and return the HKDF-SHA256
    /// output using the DH value as the HKDF ikm
    fn ec_diffie_hellman_hkdf_sha256(
        &mut self,
        context: SecretKeyContext,
        peer_public_key: PublicKey,
        salt: SecretKeyContext,
        info: &[u8],
        output_attributes: Vec<SecretKeyAttributes>,
    ) -> Result<Vec<SecretKeyContext>, VaultFailError>;
    /// Compute the HKDF-SHA256 using the specified salt and input key material
    /// and return the output key material of the specified length
    fn hkdf_sha256(
        &mut self,
        salt: SecretKeyContext,
        info: &[u8],
        ikm: Option<SecretKeyContext>,
        output_attributes: Vec<SecretKeyAttributes>,
    ) -> Result<Vec<SecretKeyContext>, VaultFailError>;
    /// Encrypt a payload using AES-GCM
    fn aead_aes_gcm_encrypt(
        &mut self,
        context: SecretKeyContext,
        plaintext: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> Result<Vec<u8>, VaultFailError>;
    /// Decrypt a payload using AES-GCM
    fn aead_aes_gcm_decrypt(
        &mut self,
        context: SecretKeyContext,
        cipher_text: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> Result<Vec<u8>, VaultFailError>;
    /// Close and release all resources in use by the vault
    fn deinit(&mut self);
    /// Generate a signature
    fn sign(
        &mut self,
        secret_key: SecretKeyContext,
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
    ) -> Result<SecretKeyContext, VaultFailError> {
        Vault::secret_generate(self, attributes)
    }

    fn secret_import(
        &mut self,
        secret: &SecretKey,
        attributes: SecretKeyAttributes,
    ) -> Result<SecretKeyContext, VaultFailError> {
        Vault::secret_import(self, secret, attributes)
    }

    fn secret_export(&mut self, context: SecretKeyContext) -> Result<SecretKey, VaultFailError> {
        Vault::secret_export(self, context)
    }

    fn secret_attributes_get(
        &mut self,
        context: SecretKeyContext,
    ) -> Result<SecretKeyAttributes, VaultFailError> {
        Vault::secret_attributes_get(self, context)
    }

    fn secret_public_key_get(
        &mut self,
        context: SecretKeyContext,
    ) -> Result<PublicKey, VaultFailError> {
        Vault::secret_public_key_get(self, context)
    }

    fn secret_destroy(&mut self, context: SecretKeyContext) -> Result<(), VaultFailError> {
        Vault::secret_destroy(self, context)
    }

    fn ec_diffie_hellman(
        &mut self,
        context: SecretKeyContext,
        peer_public_key: PublicKey,
    ) -> Result<SecretKeyContext, VaultFailError> {
        Vault::ec_diffie_hellman(self, context, peer_public_key)
    }

    fn ec_diffie_hellman_hkdf_sha256(
        &mut self,
        context: SecretKeyContext,
        peer_public_key: PublicKey,
        salt: SecretKeyContext,
        info: &[u8],
        output_attributes: Vec<SecretKeyAttributes>,
    ) -> Result<Vec<SecretKeyContext>, VaultFailError> {
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
        salt: SecretKeyContext,
        info: &[u8],
        ikm: Option<SecretKeyContext>,
        output_attributes: Vec<SecretKeyAttributes>,
    ) -> Result<Vec<SecretKeyContext>, VaultFailError> {
        Vault::hkdf_sha256(self, salt, info, ikm, output_attributes)
    }

    fn aead_aes_gcm_encrypt(
        &mut self,
        context: SecretKeyContext,
        plaintext: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> Result<Vec<u8>, VaultFailError> {
        Vault::aead_aes_gcm_encrypt(self, context, plaintext, nonce, aad)
    }

    fn aead_aes_gcm_decrypt(
        &mut self,
        context: SecretKeyContext,
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
        secret_key: SecretKeyContext,
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
