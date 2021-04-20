use crate::{
    VaultRequestMessage, VaultResponseMessage, VaultSync, VaultSyncCoreError, VaultSyncState,
};
use ockam_core::Result;
use ockam_node::block_future;
use ockam_vault_core::{Secret, Signer};

impl Signer for VaultSync {
    fn sign(&mut self, secret_key: &Secret, data: &[u8]) -> Result<[u8; 64]> {
        match &mut self.0 {
            VaultSyncState::Worker { state } => block_future(&state.ctx().runtime(), async move {
                state
                    .send_message(VaultRequestMessage::Sign {
                        secret_key: secret_key.clone(),
                        data: data.into(),
                    })
                    .await?;

                let resp = state.receive_message().await?;

                if let VaultResponseMessage::Sign(s) = resp {
                    Ok(s)
                } else {
                    Err(VaultSyncCoreError::InvalidResponseType.into())
                }
            }),
            VaultSyncState::Mutex { mutex } => mutex.lock().unwrap().sign(secret_key, data),
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
    fn sign() {}
}
