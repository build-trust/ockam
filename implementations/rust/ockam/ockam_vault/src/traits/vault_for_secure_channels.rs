use crate::{
    AeadSecretKeyHandle, HashOutput, HkdfOutput, SecretBufferHandle, X25519PublicKey,
    X25519SecretKeyHandle,
};

use ockam_core::compat::vec::Vec;
use ockam_core::{async_trait, compat::boxed::Box, Result};

/// Possible number of outputs of HKDF.
pub enum HKDFNumberOfOutputs {
    /// Derive 2 secrets.
    Two,
    /// Derive 3 secrets.
    Three,
}

/// Vault for running a Secure Channel
#[async_trait]
pub trait VaultForSecureChannels: Send + Sync + 'static {
    /// Perform X25519 ECDH.
    /// [1]: http://www.noiseprotocol.org/noise.html#dh-functions
    async fn x25519_ecdh(
        &self,
        secret_key_handle: &X25519SecretKeyHandle,
        peer_public_key: &X25519PublicKey,
    ) -> Result<SecretBufferHandle>;

    /// Compute Hash.
    /// [1]: http://www.noiseprotocol.org/noise.html#hash-functions
    async fn hash(&self, data: &[u8]) -> Result<HashOutput>;

    /// Compute HKDF.
    /// [1]: http://www.noiseprotocol.org/noise.html#hash-functions
    async fn hkdf(
        &self,
        salt: &SecretBufferHandle,
        input_key_material: Option<&SecretBufferHandle>,
        number_of_outputs: HKDFNumberOfOutputs,
    ) -> Result<HkdfOutput>;

    /// Perform AEAD encryption.
    /// [1]: http://www.noiseprotocol.org/noise.html#cipher-functions
    async fn aead_encrypt(
        &self,
        secret_key_handle: &AeadSecretKeyHandle,
        plain_text: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> Result<Vec<u8>>;

    /// Perform AEAD decryption.
    /// [1]: http://www.noiseprotocol.org/noise.html#cipher-functions
    async fn aead_decrypt(
        &self,
        secret_key_handle: &AeadSecretKeyHandle,
        cipher_text: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> Result<Vec<u8>>;

    /// Generate a fresh static (persisted) X25519 Key.
    async fn generate_static_x25519_secret_key(&self) -> Result<X25519SecretKeyHandle>;

    /// Delete static X25519 Key.
    async fn delete_static_x25519_secret_key(
        &self,
        secret_key_handle: X25519SecretKeyHandle,
    ) -> Result<bool>;

    /// Generate a fresh ephemeral (not persisted) X25519 Key.
    async fn generate_ephemeral_x25519_secret_key(&self) -> Result<X25519SecretKeyHandle>;

    /// Delete ephemeral X25519 Key.
    async fn delete_ephemeral_x25519_secret_key(
        &self,
        secret_key_handle: X25519SecretKeyHandle,
    ) -> Result<bool>;

    /// Get [`X25519PublicKey`] of the corresponding X25519 Secret Key given its Handle.
    async fn get_x25519_public_key(
        &self,
        secret_key_handle: &X25519SecretKeyHandle,
    ) -> Result<X25519PublicKey>;

    /// Get Handle to a X25519 Secret Key given its [`X25519PublicKey`].
    async fn get_x25519_secret_key_handle(
        &self,
        public_key: &X25519PublicKey,
    ) -> Result<X25519SecretKeyHandle>;

    /// Import a Secret Buffer.
    async fn import_secret_buffer(&self, buffer: Vec<u8>) -> Result<SecretBufferHandle>;

    /// Delete Secret Buffer.
    async fn delete_secret_buffer(&self, secret_buffer_handle: SecretBufferHandle) -> Result<bool>;

    /// Convert a Secret Buffer to an AEAD Key.
    async fn convert_secret_buffer_to_aead_key(
        &self,
        secret_buffer_handle: SecretBufferHandle,
    ) -> Result<AeadSecretKeyHandle>;

    /// Delete AEAD Key.
    async fn delete_aead_secret_key(&self, secret_key_handle: AeadSecretKeyHandle) -> Result<bool>;
}
