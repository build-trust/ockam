use crate::identity::IdentityError;
use crate::secure_channel::encryptor::Encryptor;
use ockam_core::compat::sync::Arc;
use ockam_core::compat::vec::Vec;
use ockam_core::vault::{KeyId, SymmetricVault};
use ockam_core::Result;

#[derive(Clone)]
pub(crate) struct Decryptor {
    key: KeyId,
    vault: Arc<dyn SymmetricVault>,
}

impl Decryptor {
    /// Restore 12-byte nonce needed for AES GCM from 8 byte that we use for noise
    fn convert_nonce_from_small(b: &[u8]) -> Result<[u8; 12]> {
        let bytes: [u8; 8] = b.try_into().map_err(|_| IdentityError::InvalidNonce)?;

        let nonce = u64::from_be_bytes(bytes);

        Ok(Encryptor::convert_nonce_from_u64(nonce).1)
    }

    pub async fn decrypt(&self, payload: &[u8]) -> Result<Vec<u8>> {
        if payload.len() < 8 {
            return Err(IdentityError::InvalidNonce.into());
        }

        let nonce = Self::convert_nonce_from_small(&payload[..8])?;

        self.vault
            .aead_aes_gcm_decrypt(&self.key, &payload[8..], &nonce, &[])
            .await
    }

    pub fn new(key: KeyId, vault: Arc<dyn SymmetricVault>) -> Self {
        Self { key, vault }
    }
}
