use ockam_core::compat::sync::Arc;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{Error, Result};
use ockam_vault::{AeadSecretKeyHandle, VaultForSecureChannels};
use tracing_attributes::instrument;

use crate::secure_channel::handshake::handshake::AES_GCM_TAGSIZE;
use crate::{Nonce, MAX_NONCE, NOISE_NONCE_LEN};

pub(crate) struct Encryptor {
    key: AeadSecretKeyHandle,
    nonce: Nonce,
    vault: Arc<dyn VaultForSecureChannels>,
    rekeying: bool,
}

// To simplify the implementation, we use the same constant for the size of the message
// window we accept with the message period used to rekey.
// This means we only need to keep the current key and the previous one.
pub(crate) const KEY_RENEWAL_INTERVAL: u64 = 32;

impl Encryptor {
    #[instrument(skip_all)]
    pub async fn rekey(
        vault: &Arc<dyn VaultForSecureChannels>,
        key: &AeadSecretKeyHandle,
    ) -> Result<AeadSecretKeyHandle> {
        let mut new_key_buffer = vec![0u8; 32 + AES_GCM_TAGSIZE];
        vault
            .aead_encrypt(
                key,
                new_key_buffer.as_mut_slice(),
                &MAX_NONCE.to_aes_gcm_nonce(),
                &[],
            )
            .await?;

        let buffer = vault
            .import_secret_buffer(new_key_buffer[0..32].to_vec())
            .await?;

        vault.convert_secret_buffer_to_aead_key(buffer).await
    }

    #[instrument(skip_all)]
    pub async fn encrypt(&mut self, payload: &mut [u8]) -> Result<()> {
        let current_nonce = self.nonce;

        self.nonce.increment()?;

        if self.rekeying
            && current_nonce.value() > 0
            && current_nonce.value() % KEY_RENEWAL_INTERVAL == 0
        {
            let new_key = Self::rekey(&self.vault, &self.key).await?;
            let old_key = core::mem::replace(&mut self.key, new_key);
            self.vault.delete_aead_secret_key(old_key).await?;
        }

        payload[..NOISE_NONCE_LEN].copy_from_slice(&current_nonce.to_noise_nonce());

        self.vault
            .aead_encrypt(
                &self.key,
                &mut payload[NOISE_NONCE_LEN..],
                &current_nonce.to_aes_gcm_nonce(),
                &[],
            )
            .await?;

        Ok(())
    }

    pub fn new(
        key: AeadSecretKeyHandle,
        nonce: Nonce,
        vault: Arc<dyn VaultForSecureChannels>,
        rekeying: bool,
    ) -> Self {
        Self {
            key,
            nonce,
            vault,
            rekeying,
        }
    }

    #[instrument(skip_all)]
    pub(crate) async fn shutdown(&self) -> Result<()> {
        if !self.vault.delete_aead_secret_key(self.key.clone()).await? {
            Err(Error::new(
                Origin::Ockam,
                Kind::Internal,
                format!(
                    "the key id {} could not be deleted in the Encryptor shutdown",
                    hex::encode(self.key.0 .0.value())
                ),
            ))
        } else {
            Ok(())
        }
    }
}
