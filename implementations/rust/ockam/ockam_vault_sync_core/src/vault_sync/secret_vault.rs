use ockam_core::vault::{PublicKey, Secret, SecretAttributes, SecretKey, SecretVault};
use ockam_core::Result;
use ockam_core::{async_trait, compat::boxed::Box};

use crate::{VaultRequestMessage, VaultResponseMessage, VaultSync, VaultSyncCoreError};

#[async_trait]
impl SecretVault for VaultSync {
    async fn secret_generate(&mut self, attributes: SecretAttributes) -> Result<Secret> {
        let resp = self
            .call(VaultRequestMessage::SecretGenerate { attributes })
            .await?;

        if let VaultResponseMessage::SecretGenerate(s) = resp {
            Ok(s)
        } else {
            Err(VaultSyncCoreError::InvalidResponseType.into())
        }
    }

    async fn secret_import(
        &mut self,
        secret: &[u8],
        attributes: SecretAttributes,
    ) -> Result<Secret> {
        let resp = self
            .call(VaultRequestMessage::SecretImport {
                secret: secret.into(),
                attributes,
            })
            .await?;

        if let VaultResponseMessage::SecretImport(s) = resp {
            Ok(s)
        } else {
            Err(VaultSyncCoreError::InvalidResponseType.into())
        }
    }

    async fn secret_export(&mut self, context: &Secret) -> Result<SecretKey> {
        let resp = self
            .call(VaultRequestMessage::SecretExport {
                context: context.clone(),
            })
            .await?;

        if let VaultResponseMessage::SecretExport(s) = resp {
            Ok(s)
        } else {
            Err(VaultSyncCoreError::InvalidResponseType.into())
        }
    }

    async fn secret_attributes_get(&mut self, context: &Secret) -> Result<SecretAttributes> {
        let resp = self
            .call(VaultRequestMessage::SecretAttributesGet {
                context: context.clone(),
            })
            .await?;

        if let VaultResponseMessage::SecretAttributesGet(s) = resp {
            Ok(s)
        } else {
            Err(VaultSyncCoreError::InvalidResponseType.into())
        }
    }

    async fn secret_public_key_get(&mut self, context: &Secret) -> Result<PublicKey> {
        let resp = self
            .call(VaultRequestMessage::SecretPublicKeyGet {
                context: context.clone(),
            })
            .await?;

        if let VaultResponseMessage::SecretPublicKeyGet(s) = resp {
            Ok(s)
        } else {
            Err(VaultSyncCoreError::InvalidResponseType.into())
        }
    }

    async fn secret_destroy(&mut self, context: Secret) -> Result<()> {
        let resp = self
            .call(VaultRequestMessage::SecretDestroy {
                context: context.clone(),
            })
            .await?;

        if let VaultResponseMessage::SecretDestroy = resp {
            Ok(())
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
    fn new_public_keys() {}

    #[ockam_macros::vault_test_sync]
    fn new_secret_keys() {}

    #[ockam_macros::vault_test_sync]
    fn secret_import_export() {}

    #[ockam_macros::vault_test_sync]
    fn secret_attributes_get() {}
}
