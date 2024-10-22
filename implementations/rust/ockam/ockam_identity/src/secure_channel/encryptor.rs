use ockam_core::compat::sync::Arc;
use ockam_core::compat::vec::Vec;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{Error, Result};
use ockam_vault::{AeadSecretKeyHandle, VaultForSecureChannels};
use tracing_attributes::instrument;

use crate::Nonce;

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
pub(crate) const SIZE_OF_NONCE: usize = 8;
pub(crate) const SIZE_OF_TAG: usize = 16;
pub(crate) const SIZE_OF_ENCRYPT_OVERHEAD: usize = SIZE_OF_NONCE + SIZE_OF_TAG;

impl Encryptor {
    #[instrument(skip_all)]
    pub async fn encrypt(&mut self, destination: &mut Vec<u8>, payload: &[u8]) -> Result<()> {
        let current_nonce = self.nonce;

        self.nonce.increment()?;

        if self.rekeying
            && current_nonce.value() > 0
            && current_nonce.value() % KEY_RENEWAL_INTERVAL == 0
        {
            let new_key = self.vault.rekey(&self.key, 1).await?;
            let old_key = core::mem::replace(&mut self.key, new_key);
            self.vault.delete_aead_secret_key(old_key).await?;
        }

        destination.extend_from_slice(&current_nonce.to_noise_nonce());

        self.vault
            .aead_encrypt(
                destination,
                &self.key,
                payload,
                &current_nonce.to_aes_gcm_nonce(),
                &[],
            )
            .await?;

        Ok(())
    }

    #[instrument(skip_all)]
    pub async fn manual_rekey(&mut self) -> Result<()> {
        let new_key = self.vault.rekey(&self.key, 1).await?;
        let old_key = core::mem::replace(&mut self.key, new_key);
        self.vault.delete_aead_secret_key(old_key).await?;
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
