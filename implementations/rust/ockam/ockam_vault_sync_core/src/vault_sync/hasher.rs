use ockam_core::vault::{Hasher, Secret, SecretAttributes, SmallBuffer};
use ockam_core::Result;
use ockam_core::{async_trait, compat::boxed::Box};

use crate::{VaultRequestMessage, VaultResponseMessage, VaultSync, VaultSyncCoreError};

#[async_trait]
impl Hasher for VaultSync {
    async fn sha256(&mut self, data: &[u8]) -> Result<[u8; 32]> {
        let resp = self
            .call(VaultRequestMessage::Sha256 { data: data.into() })
            .await?;

        if let VaultResponseMessage::Sha256(s) = resp {
            Ok(s)
        } else {
            Err(VaultSyncCoreError::InvalidResponseType.into())
        }
    }

    async fn hkdf_sha256(
        &mut self,
        salt: &Secret,
        info: &[u8],
        ikm: Option<&Secret>,
        output_attributes: SmallBuffer<SecretAttributes>,
    ) -> Result<SmallBuffer<Secret>> {
        let resp = self
            .call(VaultRequestMessage::HkdfSha256 {
                salt: salt.clone(),
                info: info.into(),
                ikm: ikm.cloned(),
                output_attributes,
            })
            .await?;

        if let VaultResponseMessage::HkdfSha256(s) = resp {
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
    fn sha256() {}

    #[ockam_macros::vault_test_sync]
    fn hkdf() {}
}
