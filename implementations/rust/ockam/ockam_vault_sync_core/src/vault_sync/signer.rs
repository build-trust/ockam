use crate::{VaultRequestMessage, VaultResponseMessage, VaultSync, VaultSyncCoreError};
use ockam_core::compat::boxed::Box;
use ockam_core::Result;
use ockam_node::block_future;
use ockam_vault_core::{Secret, Signature, Signer};

use ockam_core::async_trait::async_trait;
#[async_trait]
impl Signer for VaultSync {
    fn sign(&mut self, secret_key: &Secret, data: &[u8]) -> Result<Signature> {
        block_future(&self.ctx.runtime(), async move {
            self.send_message(VaultRequestMessage::Sign {
                secret_key: secret_key.clone(),
                data: data.into(),
            })
            .await?;

            let resp = self.receive_message().await?;

            if let VaultResponseMessage::Sign(s) = resp {
                Ok(s)
            } else {
                Err(VaultSyncCoreError::InvalidResponseType.into())
            }
        })
    }

    async fn async_sign(&mut self, secret_key: &Secret, data: &[u8]) -> Result<Signature> {
        self.send_message(VaultRequestMessage::Sign {
            secret_key: secret_key.clone(),
            data: data.into(),
        })
        .await?;

        let resp = self.receive_message().await?;

        if let VaultResponseMessage::Sign(s) = resp {
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
    fn sign() {}
}
