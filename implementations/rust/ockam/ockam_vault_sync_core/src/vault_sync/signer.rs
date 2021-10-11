use ockam_core::Result;
use ockam_vault_core::{Secret, Signature, Signer};

use crate::{VaultRequestMessage, VaultResponseMessage, VaultSync, VaultSyncCoreError};

impl Signer for VaultSync {
    fn sign(&mut self, secret_key: &Secret, data: &[u8]) -> Result<Signature> {
        let resp = self.call(VaultRequestMessage::Sign {
            secret_key: secret_key.clone(),
            data: data.into(),
        })?;

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
