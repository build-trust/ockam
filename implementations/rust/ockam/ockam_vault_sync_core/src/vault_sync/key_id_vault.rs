use crate::{VaultRequestMessage, VaultResponseMessage, VaultSync, VaultSyncCoreError};
use ockam_core::compat::{boxed::Box, string::ToString};
use ockam_core::Result;
use ockam_node::block_future;
use ockam_vault_core::{KeyId, KeyIdVault, PublicKey, Secret};

use ockam_core::async_trait::async_trait;
#[async_trait]
impl KeyIdVault for VaultSync {
    fn get_secret_by_key_id(&mut self, key_id: &str) -> Result<Secret> {
        block_future(&self.ctx.runtime(), async move {
            self.async_get_secret_by_key_id(key_id).await
        })
    }

    async fn async_get_secret_by_key_id(&mut self, key_id: &str) -> Result<Secret> {
        self.send_message(VaultRequestMessage::GetSecretByKeyId {
            key_id: key_id.to_string(),
        })
        .await?;

        let resp = self.receive_message().await?;

        if let VaultResponseMessage::GetSecretByKeyId(s) = resp {
            Ok(s)
        } else {
            Err(VaultSyncCoreError::InvalidResponseType.into())
        }
    }

    fn compute_key_id_for_public_key(&mut self, public_key: &PublicKey) -> Result<KeyId> {
        block_future(&self.ctx.runtime(), async move {
            self.async_compute_key_id_for_public_key(public_key).await
        })
    }

    async fn async_compute_key_id_for_public_key(
        &mut self,
        public_key: &PublicKey,
    ) -> Result<KeyId> {
        self.send_message(VaultRequestMessage::ComputeKeyIdForPublicKey {
            public_key: public_key.clone(),
        })
        .await?;

        let resp = self.receive_message().await?;

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
    use ockam_vault_test_attribute::*;

    fn new_vault() -> SoftwareVault {
        SoftwareVault::default()
    }

    #[vault_test_sync]
    fn compute_key_id_for_public_key() {}

    #[vault_test_sync]
    fn get_secret_by_key_id() {}
}
