use ockam_core::vault::{Secret, Signature, Signer};
use ockam_core::Result;
use ockam_core::{async_trait, compat::boxed::Box};

use crate::{VaultRequestMessage, VaultResponseMessage, VaultSync, VaultSyncCoreError};

#[async_trait]
impl Signer for VaultSync {
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

    fn new_vault() -> SoftwareVault {
        SoftwareVault::default()
    }

    #[ockam_macros::vault_test_sync]
    fn sign() {}
}
