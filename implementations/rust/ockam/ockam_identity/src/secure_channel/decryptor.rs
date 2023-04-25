use crate::identity::IdentityError;
use crate::secure_channel::encryptor::{Encryptor, KEY_RENEWAL_INTERVAL};
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

pub(crate) struct NonceTracker {
    nonce_bitmap: u16,
    current_nonce: u64,
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

        self.nonce_tracker.mark(nonce)?;

        if nonce >= self.current_key_nonce + KEY_RENEWAL_INTERVAL {
            //we need to rekey
            if let Some(previous_key) = self.previous_key.replace(self.current_key.clone()) {
                self.vault.secret_destroy(previous_key).await?;
            }

            self.current_key = Encryptor::rekey(&self.vault, &self.current_key).await?;
            self.current_key_nonce = nonce - nonce % KEY_RENEWAL_INTERVAL;
        }

        let key = if nonce >= self.current_key_nonce {
            &self.current_key
        } else {
            if let Some(key) = &self.previous_key {
                key
            } else {
                //shouldn't happen since nonce_tracker should reject such messages
                warn!("invalid nonce for previous key");
                return Err(IdentityError::InvalidNonce.into());
            }
        };

        self.vault
            .aead_aes_gcm_decrypt(&key, &payload[8..], &nonce_buffer, &[])
            .await
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

impl NonceTracker {
    pub(crate) fn new() -> Self {
        Self {
            nonce_bitmap: 0,
            current_nonce: 0,
        }
    }

    /// Mark a nonce as received, reject all invalid nonce values
    pub(crate) fn mark(&mut self, nonce: u64) -> Result<()> {
        if nonce >= u64::MAX - KEY_RENEWAL_INTERVAL {
            return Err(IdentityError::InvalidNonce.into());
        }

        if nonce > self.current_nonce {
            // normal case, we increase the nonce and move the window
            let relative_shift: u64 = nonce - self.current_nonce;
            if relative_shift > KEY_RENEWAL_INTERVAL {
                return Err(IdentityError::InvalidNonce.into());
            }

            self.nonce_bitmap <<= relative_shift;
            self.nonce_bitmap |= 1;
            self.current_nonce = nonce;
        } else {
            // we received a message from the past, out of order
            let relative: u64 = self.current_nonce - nonce;
            if relative > KEY_RENEWAL_INTERVAL {
                return Err(IdentityError::InvalidNonce.into());
            }

            let bit = 1 << relative;
            if self.nonce_bitmap & bit != 0 {
                // we already processed this nonce
                return Err(IdentityError::InvalidNonce.into());
            }
            self.nonce_bitmap |= bit;
        }

        Ok(())
    }
}

#[test]
pub fn check_nonce_tracker() {
    let mut tracker = NonceTracker::new();
    tracker.mark(0).unwrap();
    tracker.mark(1).unwrap();
    tracker.mark(12).unwrap_err();
    tracker.mark(11).unwrap();
    tracker.mark(1).unwrap_err();
    tracker.mark(2).unwrap();
    tracker.mark(3).unwrap();
    tracker.mark(2).unwrap_err();
    tracker.mark(20).unwrap();
    tracker.mark(9).unwrap_err();
}
