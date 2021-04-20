use crate::{
    VaultRequestMessage, VaultResponseMessage, VaultSync, VaultSyncCoreError, VaultSyncState,
};
use ockam_core::Result;
use ockam_node::block_future;
use ockam_vault_core::{PublicKey, Secret, SecretAttributes, SecretKey, SecretVault};

impl SecretVault for VaultSync {
    fn secret_generate(&mut self, attributes: SecretAttributes) -> Result<Secret> {
        match &mut self.0 {
            VaultSyncState::Worker { state } => block_future(&state.ctx().runtime(), async move {
                state
                    .send_message(VaultRequestMessage::SecretGenerate { attributes })
                    .await?;

                let resp = state.receive_message().await?;

                if let VaultResponseMessage::SecretGenerate(s) = resp {
                    Ok(s)
                } else {
                    Err(VaultSyncCoreError::InvalidResponseType.into())
                }
            }),
            VaultSyncState::Mutex { mutex } => mutex.lock().unwrap().secret_generate(attributes),
        }
    }

    fn secret_import(&mut self, secret: &[u8], attributes: SecretAttributes) -> Result<Secret> {
        match &mut self.0 {
            VaultSyncState::Worker { state } => block_future(&state.ctx().runtime(), async move {
                state
                    .send_message(VaultRequestMessage::SecretImport {
                        secret: secret.into(),
                        attributes,
                    })
                    .await?;

                let resp = state.receive_message().await?;

                if let VaultResponseMessage::SecretImport(s) = resp {
                    Ok(s)
                } else {
                    Err(VaultSyncCoreError::InvalidResponseType.into())
                }
            }),
            VaultSyncState::Mutex { mutex } => {
                mutex.lock().unwrap().secret_import(secret, attributes)
            }
        }
    }

    fn secret_export(&mut self, context: &Secret) -> Result<SecretKey> {
        match &mut self.0 {
            VaultSyncState::Worker { state } => block_future(&state.ctx().runtime(), async move {
                state
                    .send_message(VaultRequestMessage::SecretExport {
                        context: context.clone(),
                    })
                    .await?;

                let resp = state.receive_message().await?;

                if let VaultResponseMessage::SecretExport(s) = resp {
                    Ok(s)
                } else {
                    Err(VaultSyncCoreError::InvalidResponseType.into())
                }
            }),
            VaultSyncState::Mutex { mutex } => mutex.lock().unwrap().secret_export(context),
        }
    }

    fn secret_attributes_get(&mut self, context: &Secret) -> Result<SecretAttributes> {
        match &mut self.0 {
            VaultSyncState::Worker { state } => block_future(&state.ctx().runtime(), async move {
                state
                    .send_message(VaultRequestMessage::SecretAttributesGet {
                        context: context.clone(),
                    })
                    .await?;

                let resp = state.receive_message().await?;

                if let VaultResponseMessage::SecretAttributesGet(s) = resp {
                    Ok(s)
                } else {
                    Err(VaultSyncCoreError::InvalidResponseType.into())
                }
            }),
            VaultSyncState::Mutex { mutex } => mutex.lock().unwrap().secret_attributes_get(context),
        }
    }

    fn secret_public_key_get(&mut self, context: &Secret) -> Result<PublicKey> {
        match &mut self.0 {
            VaultSyncState::Worker { state } => block_future(&state.ctx().runtime(), async move {
                state
                    .send_message(VaultRequestMessage::SecretPublicKeyGet {
                        context: context.clone(),
                    })
                    .await?;

                let resp = state.receive_message().await?;

                if let VaultResponseMessage::SecretPublicKeyGet(s) = resp {
                    Ok(s)
                } else {
                    Err(VaultSyncCoreError::InvalidResponseType.into())
                }
            }),
            VaultSyncState::Mutex { mutex } => mutex.lock().unwrap().secret_public_key_get(context),
        }
    }

    fn secret_destroy(&mut self, context: Secret) -> Result<()> {
        match &mut self.0 {
            VaultSyncState::Worker { state } => block_future(&state.ctx().runtime(), async move {
                state
                    .send_message(VaultRequestMessage::SecretDestroy {
                        context: context.clone(),
                    })
                    .await?;

                let resp = state.receive_message().await?;

                if let VaultResponseMessage::SecretDestroy = resp {
                    Ok(())
                } else {
                    Err(VaultSyncCoreError::InvalidResponseType.into())
                }
            }),
            VaultSyncState::Mutex { mutex } => mutex.lock().unwrap().secret_destroy(context),
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
    fn new_public_keys() {}

    #[vault_test_sync]
    fn new_secret_keys() {}

    #[vault_test_sync]
    fn secret_import_export() {}

    #[vault_test_sync]
    fn secret_attributes_get() {}
}
