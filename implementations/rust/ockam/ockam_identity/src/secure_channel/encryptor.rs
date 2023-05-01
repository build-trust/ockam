use crate::identity::IdentityError;
use ockam_core::compat::sync::Arc;
use ockam_core::compat::vec::Vec;
use ockam_core::vault::{KeyId, Secret, SecretKey};
use ockam_core::Result;
use ockam_key_exchange_xx::XXInitializedVault;

pub(crate) struct Encryptor {
    key: KeyId,
    nonce: u64,
    vault: Arc<dyn XXInitializedVault>,
}

// To simplify the implementation we use the same constant for the size of the message
// window we accept with the message period used to rekey.
// This means we only need to keep the current key and the previous one.
pub(crate) const KEY_RENEWAL_INTERVAL: u64 = 32;

impl Encryptor {
    /// We use u64 nonce since it's convenient to work with it (e.g. increment)
    /// But we use 8-byte be format to send it over to the other side (according to noise spec)
    /// And we use 12-byte be format for encryption, since AES-GCM wants 12 bytes
    pub(crate) fn convert_nonce_from_u64(nonce: u64) -> ([u8; 8], [u8; 12]) {
        let mut n: [u8; 12] = [0; 12];
        let b: [u8; 8] = nonce.to_be_bytes();

        n[4..].copy_from_slice(&b);

        (b, n)
    }

    pub async fn rekey(vault: &Arc<dyn XXInitializedVault>, key: &KeyId) -> Result<KeyId> {
        let nonce_buffer = Self::convert_nonce_from_u64(u64::MAX).1;
        let zeroes = [0u8; 32];

        let new_key_buffer = vault
            .aead_aes_gcm_encrypt(key, &zeroes, &nonce_buffer, &[])
            .await?;

        let attributes = vault.secret_attributes_get(key).await?;

        vault
            .secret_import(
                Secret::Key(SecretKey::new(new_key_buffer[0..32].to_vec())),
                attributes,
            )
            .await
    }

    pub async fn encrypt(&mut self, payload: &[u8]) -> Result<Vec<u8>> {
        let current_nonce = self.nonce;
        if current_nonce == u64::MAX {
            return Err(IdentityError::NonceOverflow.into());
        }

        self.nonce += 1;

        if current_nonce > 0 && current_nonce % KEY_RENEWAL_INTERVAL == 0 {
            let new_key = Self::rekey(&self.vault, &self.key).await?;
            let old_key = core::mem::replace(&mut self.key, new_key);
            self.vault.secret_destroy(old_key).await?;
        }

        let (small_nonce, nonce) = Self::convert_nonce_from_u64(current_nonce);

        let mut cipher_text = self
            .vault
            .aead_aes_gcm_encrypt(&self.key, payload, &nonce, &[])
            .await?;

        let mut res = Vec::new();
        res.extend_from_slice(&small_nonce);
        res.append(&mut cipher_text);

        Ok(res)
    }

    pub fn new(key: KeyId, nonce: u64, vault: Arc<dyn XXInitializedVault>) -> Self {
        Self { key, nonce, vault }
    }
}
