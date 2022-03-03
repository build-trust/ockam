use crate::software_vault::SoftwareVault;
use crate::VaultError;
use ockam_core::vault::{Hasher, KeyId, KeyIdVault, PublicKey, Secret};
use ockam_core::Result;
use ockam_core::{async_trait, compat::boxed::Box};

#[async_trait]
impl KeyIdVault for SoftwareVault {
    async fn secret_by_key_id(&mut self, key_id: &str) -> Result<Secret> {
        let index = self
            .data
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

    async fn compute_key_id_for_public_key(&mut self, public_key: &PublicKey) -> Result<KeyId> {
        let key_id = self.sha256(public_key.as_ref()).await?;
        Ok(hex::encode(key_id))
    }
}

#[cfg(test)]
mod tests {
    use crate::SoftwareVault;

    fn new_vault() -> SoftwareVault {
        SoftwareVault::default()
    }

    #[ockam_macros::vault_test]
    fn compute_key_id_for_public_key() {}

    #[ockam_macros::vault_test]
    fn secret_by_key_id() {}
}
