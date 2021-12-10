use ockam_core::compat::string::ToString;
use ockam_core::vault::{KeyId, KeyIdVault, PublicKey, Secret};
use ockam_core::Result;
use ockam_core::{async_trait, compat::boxed::Box};

use crate::{VaultRequestMessage, VaultResponseMessage, VaultSync, VaultSyncCoreError};

#[async_trait]
impl KeyIdVault for VaultSync {
    async fn get_secret_by_key_id(&mut self, key_id: &str) -> Result<Secret> {
        let resp = self
            .call(VaultRequestMessage::GetSecretByKeyId {
                key_id: key_id.to_string(),
            })
            .await?;

        if let VaultResponseMessage::GetSecretByKeyId(s) = resp {
            Ok(s)
        } else {
            Err(VaultSyncCoreError::InvalidResponseType.into())
        }
    }

    async fn compute_key_id_for_public_key(&mut self, public_key: &PublicKey) -> Result<KeyId> {
        let resp = self
            .call(VaultRequestMessage::ComputeKeyIdForPublicKey {
                public_key: public_key.clone(),
            })
            .await?;

        if let VaultResponseMessage::ComputeKeyIdForPublicKey(s) = resp {
            Ok(s)
        } else {
            Err(VaultSyncCoreError::InvalidResponseType.into())
        }
    }
}

#[cfg(test)]
mod tests {
    use ockam_vault::SoftwareVault;

    fn new_vault() -> SoftwareVault {
        SoftwareVault::default()
    }

    #[ockam_macros::vault_test_sync]
    fn compute_key_id_for_public_key() {}

    #[ockam_macros::vault_test_sync]
    fn get_secret_by_key_id() {}
}
