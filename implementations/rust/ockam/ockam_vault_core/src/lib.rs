//! Core types and traits of the Ockam vault.
//!
//! This crate contains the core types and traits of the Ockam vault and is intended
//! for use by other crates that either provide implementations for those traits,
//! or use traits and types as an abstract dependency.
#![deny(unsafe_code)]
#![warn(
    missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    unused_import_braces,
    unused_qualifications
)]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;

use ockam_core::Result;

mod asymmetric_vault;
mod hasher;
mod key_id_vault;
mod secret;
mod secret_vault;
mod signer;
mod symmetric_vault;
mod types;
mod verifier;

pub use asymmetric_vault::*;
pub use hasher::*;
pub use key_id_vault::*;
pub use secret::*;
pub use secret_vault::*;
pub use signer::*;
pub use symmetric_vault::*;
pub use types::*;
pub use verifier::*;



/// An abstraction over cryptographic operations.
///
/// Covers hashing, secret management, signing and signature verification,
/// symmetric and asymmetric cryptography, etc.
///
/// This trait currently requires Send/Sync, however these may be relaxed in the
/// future in some cases.
#[async_trait::async_trait]
pub trait Vault: Send + Sync {
    /// Compute Elliptic-Curve Diffie-Hellman using this secret key
    /// and the specified uncompressed public key
    async fn ec_diffie_hellman(
        &self,
        context: &Secret,
        peer_public_key: &PublicKey,
    ) -> Result<Secret>;

    /// Compute the SHA-256 digest given input `data`
    async fn sha256(&self, data: &[u8]) -> Result<[u8; 32]>;

    /// Derive multiple output [`Secret`]s with given attributes using the HKDF-SHA256 using
    /// specified salt, input key material and info.
    async fn hkdf_sha256(
        &self,
        salt: &Secret,
        info: &[u8],
        ikm: Option<&Secret>,
        output_attributes: SmallBuffer<SecretAttributes>,
    ) -> Result<SmallBuffer<Secret>>;

    /// Return [`Secret`] for given key id
    async fn get_secret_by_key_id(&self, key_id: &str) -> Result<Secret>;
    /// Return KeyId for given public key
    async fn compute_key_id_for_public_key(&self, public_key: &PublicKey) -> Result<KeyId>;

    /// Generate fresh secret with given attributes
    async fn secret_generate(&self, attributes: SecretAttributes) -> Result<Secret>;
    /// Import a secret with given attributes from binary form into the vault
    async fn secret_import(
        &self,
        secret: &[u8],
        attributes: SecretAttributes,
    ) -> Result<Secret>;

    /// Export a secret key to the binary form represented as [`SecretKey`]
    async fn secret_export(&self, context: &Secret) -> Result<SecretKey>;
    /// Get the attributes for a secret
    async fn secret_attributes_get(&self, context: &Secret) -> Result<SecretAttributes>;
    /// Return the associated public key given the secret key
    async fn secret_public_key_get(&self, context: &Secret) -> Result<PublicKey>;
    /// Remove a secret from the vault
    async fn secret_destroy(&self, context: Secret) -> Result<()>;

    /// Generate a signature for given data using given secret key
    async fn sign(&self, secret_key: &Secret, data: &[u8]) -> Result<Signature>;
    /// Verify a signature for given data using given public key
    async fn verify(
        &self,
        signature: &Signature,
        public_key: &PublicKey,
        data: &[u8],
    ) -> Result<bool>;

    /// Encrypt a payload using AES-GCM
    async fn aead_aes_gcm_encrypt(
        &self,
        context: &Secret,
        plaintext: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> Result<Buffer<u8>>;

    /// Decrypt a payload using AES-GCM
    async fn aead_aes_gcm_decrypt(
        &self,
        context: &Secret,
        cipher_text: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> Result<Buffer<u8>>;
}

/// Super-trait of traits required for an `ArcVault`.
pub trait VaultTrait:
    AsymmetricVault
    + Hasher
    + KeyIdVault
    + SecretVault
    + Signer
    + SymmetricVault
    + Verifier
    + Send
    + Sync
{
}

impl<V> VaultTrait for V where
    V: AsymmetricVault
        + Hasher
        + KeyIdVault
        + SecretVault
        + Signer
        + SymmetricVault
        + Verifier
        + Send
        + Sync
        + ?Sized
{
}

/// `ArcVault` is a convenience alias used for
pub type ArcVault<V = dyn VaultTrait + 'static> = ockam_core::compat::sync::Arc<V>;

#[test]
fn verify_impls() {
    fn check_vault_trait<T: VaultTrait>() {}
    check_vault_trait::<ArcVault>();
}
