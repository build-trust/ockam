use crate::software_vault::SoftwareVault;
use crate::VaultError;
use ockam_core::hex::encode;
use ockam_core::Result;
use ockam_vault_core::{KeyId, PublicKey, Secret};

impl SoftwareVault {
    pub(crate) fn get_secret_by_key_id_sync(&self, key_id: &str) -> Result<Secret> {
        let storage = self.inner.read();
        let index = storage
            .entries
            .iter()
            .find(|(_, entry)| {
                if let Some(e_key_id) = entry.key_id() {
                    e_key_id == key_id
                } else {
                    false
                }
            })
            .ok_or_else(|| Into::<ockam_core::Error>::into(VaultError::SecretNotFound))?
            .0;

        Ok(Secret::new(*index))
    }

    pub(crate) fn compute_key_id_for_public_key_sync(&self, public_key: &PublicKey) -> Result<KeyId> {
        let key_id = self.sha256_sync(public_key.as_ref())?;
        Ok(encode(key_id))
    }
}
