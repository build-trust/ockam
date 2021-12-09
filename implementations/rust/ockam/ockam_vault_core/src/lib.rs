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

use ockam_core::{Result, compat::{sync::Arc, boxed::Box}};
use async_trait::async_trait;

mod types;
mod secret;

pub use types::*;
pub use secret::*;

/// An abstraction over cryptographic operations.
///
/// Covers hashing, secret management, signing and signature verification,
/// symmetric and asymmetric cryptography, etc.
///
/// (In the future, it is likely that this trait will be replaced by a (larger)
/// set of simpler traits)
#[async_trait]
pub trait Vault: Send + Sync + 'static {
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

/// Implement `Vault` for a reference or smart pointer by forwarding the impl to
/// the pointee. Used to generate the impls for `Arc<V>` `Box<V>`
/// and `&'static V`, for all `V` who `impl Vault`
macro_rules! forward_vault_impl {
    ($V:ident for $Ty:ty) => {
        #[async_trait]
        impl<$V: Vault + ?Sized> Vault for $Ty {
            async fn ec_diffie_hellman(
                &self,
                context: &Secret,
                peer_public_key: &PublicKey,
            ) -> Result<Secret> {
                <$V>::ec_diffie_hellman(&**self, context, peer_public_key).await
            }

            async fn sha256(&self, data: &[u8]) -> Result<[u8; 32]> {
                <$V>::sha256(&**self, data).await
            }
            async fn hkdf_sha256(
                &self,
                salt: &Secret,
                info: &[u8],
                ikm: Option<&Secret>,
                output_attributes: SmallBuffer<SecretAttributes>,
            ) -> Result<SmallBuffer<Secret>> {
                <$V>::hkdf_sha256(&**self, salt, info, ikm, output_attributes).await
            }

            async fn get_secret_by_key_id(&self, key_id: &str) -> Result<Secret> {
                <$V>::get_secret_by_key_id(&**self, key_id).await
            }

            async fn compute_key_id_for_public_key(&self, public_key: &PublicKey) -> Result<KeyId> {
                <$V>::compute_key_id_for_public_key(&**self, public_key).await
            }

            async fn secret_generate(&self, attributes: SecretAttributes) -> Result<Secret> {
                <$V>::secret_generate(&**self, attributes).await
            }

            async fn secret_import(&self, secret: &[u8], attributes: SecretAttributes) -> Result<Secret> {
                <$V>::secret_import(&**self, secret, attributes).await
            }
            async fn secret_export(&self, context: &Secret) -> Result<SecretKey> {
                <$V>::secret_export(&**self, context).await
            }
            async fn secret_attributes_get(&self, context: &Secret) -> Result<SecretAttributes> {
                <$V>::secret_attributes_get(&**self, context).await
            }
            async fn secret_public_key_get(&self, context: &Secret) -> Result<PublicKey> {
                <$V>::secret_public_key_get(&**self, context).await
            }
            async fn secret_destroy(&self, context: Secret) -> Result<()> {
                <$V>::secret_destroy(&**self, context).await
            }

            async fn sign(&self, secret_key: &Secret, data: &[u8]) -> Result<Signature> {
                <$V>::sign(&**self, secret_key, data).await
            }
            async fn verify(
                &self,
                signature: &Signature,
                public_key: &PublicKey,
                data: &[u8],
            ) -> Result<bool> {
                <$V>::verify(&**self, signature, public_key, data).await
            }
            async fn aead_aes_gcm_encrypt(
                &self,
                context: &Secret,
                plaintext: &[u8],
                nonce: &[u8],
                aad: &[u8],
            ) -> Result<Buffer<u8>> {
                <$V>::aead_aes_gcm_encrypt(&**self, context, plaintext, nonce, aad).await
            }
            async fn aead_aes_gcm_decrypt(
                &self,
                context: &Secret,
                cipher_text: &[u8],
                nonce: &[u8],
                aad: &[u8],
            ) -> Result<Buffer<u8>> {
                <$V>::aead_aes_gcm_decrypt(&**self, context, cipher_text, nonce, aad).await
            }
        }
    };
}

forward_vault_impl!(V for &'static V);
forward_vault_impl!(V for Box<V>);
forward_vault_impl!(V for Arc<V>);

#[test]
fn verify_impls() {
    fn check_vault_trait<T: Vault>() {}
    check_vault_trait::<Box<dyn Vault>>();
    check_vault_trait::<Arc<dyn Vault>>();
    check_vault_trait::<&'static dyn Vault>();
    check_vault_trait::<&'static Box<dyn Vault>>();
}
