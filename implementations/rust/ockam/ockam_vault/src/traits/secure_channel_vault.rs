use crate::{Buffer, KeyId, PublicKey, Secret, SecretAttributes, SmallBuffer};
use ockam_core::{async_trait, compat::boxed::Box, Result};

/// Vault used for Secure Channel
#[async_trait]
pub trait SecureChannelVault: Send + Sync + 'static {
    /// Generate a fresh random secret that is persisted in the Storage
    async fn generate_static_secret(&self, attributes: SecretAttributes) -> Result<KeyId>;

    /// Generate a fresh random secret that is persisted only in memory
    async fn generate_ephemeral_secret(&self, attributes: SecretAttributes) -> Result<KeyId>;

    /// Import a secret that is persisted in the Storage
    async fn import_static_secret(
        &self,
        secret: Secret,
        attributes: SecretAttributes,
    ) -> Result<KeyId>;

    /// Import a secret that is persisted only in memory
    async fn import_ephemeral_secret(
        &self,
        secret: Secret,
        attributes: SecretAttributes,
    ) -> Result<KeyId>;

    /// Delete a Secret
    async fn delete_secret(&self, key_id: KeyId) -> Result<bool>;

    /// Get corresponding [`PublicKey`]
    async fn get_public_key(&self, key_id: &KeyId) -> Result<PublicKey>;

    /// Get corresponding [`KeyId`] given a [`PublicKey`]
    async fn get_key_id(&self, public_key: &PublicKey) -> Result<KeyId>;

    /// Get Secret's [`SecretAttributes`]
    async fn get_secret_attributes(&self, key_id: &KeyId) -> Result<SecretAttributes>;

    /// Compute and store an Elliptic-Curve Diffie-Hellman
    async fn ec_diffie_hellman(&self, secret: &KeyId, peer_public_key: &PublicKey)
        -> Result<KeyId>;

    /// Derive multiple output [`super::Secret`]s with given attributes using
    /// the HKDF-SHA256 given the specified salt, info and input key
    /// material
    async fn hkdf_sha256(
        &self,
        salt: &KeyId,
        info: &[u8],
        ikm: Option<&KeyId>,
        output_attributes: SmallBuffer<SecretAttributes>,
    ) -> Result<SmallBuffer<KeyId>>;

    /// Encrypt a payload using AES-GCM
    async fn aead_aes_gcm_encrypt(
        &self,
        key_id: &KeyId,
        plaintext: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> Result<Buffer<u8>>;

    /// Decrypt a payload using AES-GCM
    async fn aead_aes_gcm_decrypt(
        &self,
        key_id: &KeyId,
        cipher_text: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> Result<Buffer<u8>>;
}
