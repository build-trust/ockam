use crate::{Vault, VaultRequestMessage, VaultResponseMessage, VaultSyncCoreError};
use ockam_core::Result;
use ockam_node::block_future;
use ockam_vault_core::{KeyId, KeyIdVault, PublicKey, Secret};

impl KeyIdVault for Vault {
    fn get_secret_by_key_id(&mut self, key_id: &str) -> Result<Secret> {
        block_future(&self.ctx().runtime(), async move {
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
        })
    }

    fn compute_key_id_for_public_key(&mut self, public_key: &PublicKey) -> Result<KeyId> {
        block_future(&self.ctx().runtime(), async move {
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
        })
    }
}
