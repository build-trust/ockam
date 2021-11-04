use ockam_core::{async_trait, compat::boxed::Box};
use ockam_core::{NodeContext, Result};
use ockam_vault_core::{Secret, Signature, Signer};

use crate::{VaultRequestMessage, VaultResponseMessage, VaultSync, VaultSyncCoreError};

#[async_trait]
impl<C: NodeContext> Signer for VaultSync<C> {
    async fn sign(&mut self, secret_key: &Secret, data: &[u8]) -> Result<Signature> {
        let resp = self
            .call(VaultRequestMessage::Sign {
                secret_key: secret_key.clone(),
                data: data.into(),
            })
            .await?;

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
