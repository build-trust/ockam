use crate::identity::IdentityError;
use crate::secure_channel::encryptor::{Encryptor, KEY_RENEWAL_INTERVAL};
use crate::secure_channel::nonce_tracker::NonceTracker;
use ockam_core::compat::sync::Arc;
use ockam_core::compat::vec::Vec;
use ockam_core::vault::KeyId;
use ockam_core::Result;
use ockam_key_exchange_xx::XXInitializedVault;
use tracing::warn;

pub(crate) struct Decryptor {
    current_key: KeyId,
    current_key_nonce: u64,

    previous_key: Option<KeyId>,
    vault: Arc<dyn XXInitializedVault>,
    nonce_tracker: NonceTracker,
}

impl Decryptor {
    /// Restore 12-byte nonce needed for AES GCM from 8 byte that we use for noise
    fn convert_nonce_from_small(b: &[u8]) -> Result<(u64, [u8; 12])> {
        let bytes: [u8; 8] = b.try_into().map_err(|_| IdentityError::InvalidNonce)?;

        let nonce = u64::from_be_bytes(bytes);

        Ok((nonce, Encryptor::convert_nonce_from_u64(nonce).1))
    }

    pub async fn decrypt(&mut self, payload: &[u8]) -> Result<Vec<u8>> {
        if payload.len() < 8 {
            return Err(IdentityError::InvalidNonce.into());
        }

        let (nonce, nonce_buffer) = Self::convert_nonce_from_small(&payload[..8])?;

        let nonce_tracker = self.nonce_tracker.mark(nonce)?;

        // to improve protection against connection disruption attacks, we want to validate the
        // message with a decryption _before_ committing to the new state

        if nonce >= self.current_key_nonce + KEY_RENEWAL_INTERVAL {
            // we need to rekey
            let new_key = Encryptor::rekey(&self.vault, &self.current_key).await?;
            let new_key_nonce = nonce - nonce % KEY_RENEWAL_INTERVAL;

            let result = self
                .vault
                .aead_aes_gcm_decrypt(&new_key, &payload[8..], &nonce_buffer, &[])
                .await;

            if result.is_ok() {
                if let Some(previous_key) = self.previous_key.replace(self.current_key.clone()) {
                    self.vault.secret_destroy(previous_key).await?;
                }

                self.nonce_tracker = nonce_tracker;
                self.current_key = new_key;
                self.current_key_nonce = new_key_nonce;
            }

            result
        } else {
            let key = if nonce >= self.current_key_nonce {
                &self.current_key
            } else if let Some(key) = &self.previous_key {
                key
            } else {
                // shouldn't happen since nonce_tracker should reject such messages
                warn!("invalid nonce for previous key");
                return Err(IdentityError::InvalidNonce.into());
            };

            let result = self
                .vault
                .aead_aes_gcm_decrypt(key, &payload[8..], &nonce_buffer, &[])
                .await;

            if result.is_ok() {
                self.nonce_tracker = nonce_tracker;
            }

            result
        }
    }

    pub fn new(key: KeyId, vault: Arc<dyn XXInitializedVault>) -> Self {
        Self {
            current_key: key,
            current_key_nonce: 0,
            previous_key: None,
            vault,
            nonce_tracker: NonceTracker::new(),
        }
    }
}
