use ockam_core::{Result, ResultMessage};
use ockam_vault_core::{Hasher, Secret, SecretAttributes, SmallBuffer};

use crate::{VaultRequestMessage, VaultResponseMessage, VaultSync, VaultSyncCoreError};


impl Hasher for VaultSync {
    fn sha256(&mut self, data: &[u8]) -> Result<[u8; 32]> {
        let resp = self.handle
            .call::<VaultRequestMessage, ResultMessage<VaultResponseMessage>>(
                VaultRequestMessage::Sha256 { data: data.into() }
            )?.into();

        if let Ok(VaultResponseMessage::Sha256(s)) = resp {
            Ok(s)
        } else {
            Err(VaultSyncCoreError::InvalidResponseType.into())
        }
}

fn hkdf_sha256(
    &mut self,
    salt: &Secret,
    info: &[u8],
    ikm: Option<&Secret>,
    output_attributes: SmallBuffer<SecretAttributes>,
) -> Result<SmallBuffer<Secret>> {
    let resp: ResultMessage<VaultResponseMessage> = self.handle
        .call(VaultRequestMessage::HkdfSha256 {
            salt: salt.clone(),
            info: info.into(),
            ikm: ikm.cloned(),
            output_attributes,
        })?;

    if let Ok(VaultResponseMessage::HkdfSha256(s)) = resp.into() {
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
    fn sha256() {}

    #[vault_test_sync]
    fn hkdf() {}
}
