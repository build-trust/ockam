use crate::{
    VaultRequestMessage, VaultResponseMessage, VaultSync, VaultSyncCoreError, VaultSyncState,
};
use ockam_core::Result;
use ockam_node::block_future;
use ockam_vault_core::{KeyId, KeyIdVault, PublicKey, Secret};

impl KeyIdVault for VaultSync {
    fn get_secret_by_key_id(&mut self, key_id: &str) -> Result<Secret> {
        match &mut self.0 {
            VaultSyncState::Worker { state } => block_future(&state.ctx().runtime(), async move {
                state
                    .send_message(VaultRequestMessage::GetSecretByKeyId {
                        key_id: key_id.to_string(),
                    })
                    .await?;

                let resp = state.receive_message().await?;

                if let VaultResponseMessage::GetSecretByKeyId(s) = resp {
                    Ok(s)
                } else {
                    Err(VaultSyncCoreError::InvalidResponseType.into())
                }
            }),
            VaultSyncState::Mutex { mutex } => mutex.lock().unwrap().get_secret_by_key_id(key_id),
        }
    }

    fn compute_key_id_for_public_key(&mut self, public_key: &PublicKey) -> Result<KeyId> {
        match &mut self.0 {
            VaultSyncState::Worker { state } => block_future(&state.ctx().runtime(), async move {
                state
                    .send_message(VaultRequestMessage::ComputeKeyIdForPublicKey {
                        public_key: public_key.clone(),
                    })
                    .await?;

                let resp = state.receive_message().await?;

                if let VaultResponseMessage::ComputeKeyIdForPublicKey(s) = resp {
                    Ok(s)
                } else {
                    Err(VaultSyncCoreError::InvalidResponseType.into())
                }
            }),
            VaultSyncState::Mutex { mutex } => mutex
                .lock()
                .unwrap()
                .compute_key_id_for_public_key(public_key),
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
