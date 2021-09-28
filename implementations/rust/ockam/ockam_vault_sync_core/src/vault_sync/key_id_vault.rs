use ockam_core::compat::string::ToString;
use ockam_core::Result;
use ockam_vault_core::{KeyId, KeyIdVault, PublicKey, Secret};

use crate::{VaultRequestMessage, VaultResponseMessage, VaultSync, VaultSyncCoreError};

impl KeyIdVault for VaultSync {
    fn get_secret_by_key_id(&mut self, key_id: &str) -> Result<Secret> {
        let resp = self.call(VaultRequestMessage::GetSecretByKeyId {
            key_id: key_id.to_string(),
        })?;

        if let VaultResponseMessage::GetSecretByKeyId(s) = resp {
            Ok(s)
        } else {
            Err(VaultSyncCoreError::InvalidResponseType.into())
        }
    }

    fn compute_key_id_for_public_key(&mut self, public_key: &PublicKey) -> Result<KeyId> {
        let resp = self.call(VaultRequestMessage::ComputeKeyIdForPublicKey {
            public_key: public_key.clone(),
        })?;

        if let VaultResponseMessage::ComputeKeyIdForPublicKey(s) = resp {
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
    fn compute_key_id_for_public_key() {}

    #[vault_test_sync]
    fn get_secret_by_key_id() {}
}
