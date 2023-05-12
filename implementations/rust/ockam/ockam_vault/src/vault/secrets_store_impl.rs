use crate::{
    EphemeralSecretsStore, Implementation, KeyId, PersistentSecretsStore, PublicKey, Secret,
    SecretAttributes, SecretsStoreReader, SecurityModule, Signature, StoredSecret, VaultError,
    VaultSecurityModule,
};
use ockam_core::{async_trait, compat::boxed::Box, compat::sync::Arc, Result};
use ockam_node::KeyValueStorage;

#[derive(Clone)]
pub struct VaultSecretsStore {
    security_module: Arc<dyn SecurityModule>,
    ephemeral_secrets: Arc<dyn KeyValueStorage<KeyId, StoredSecret>>,
}

impl Implementation for VaultSecretsStore {}

impl VaultSecretsStore {
    pub fn new(
        security_module: Arc<dyn SecurityModule>,
        ephemeral_secrets: Arc<dyn KeyValueStorage<KeyId, StoredSecret>>,
    ) -> VaultSecretsStore {
        VaultSecretsStore {
            security_module,
            ephemeral_secrets,
        }
    }
}

#[async_trait]
impl EphemeralSecretsStore for VaultSecretsStore {
    async fn create_ephemeral_secret(&self, attributes: SecretAttributes) -> Result<KeyId> {
        let secret = VaultSecurityModule::create_secret_from_attributes(attributes)?;
        self.import_ephemeral_secret(secret, attributes).await
    }

    async fn import_ephemeral_secret(
        &self,
        secret: Secret,
        attributes: SecretAttributes,
    ) -> Result<KeyId> {
        let key_id = VaultSecurityModule::compute_key_id(&secret, &attributes).await?;
        let stored_secret = StoredSecret::create(secret, attributes)?;
        self.ephemeral_secrets
            .put(key_id.clone(), stored_secret)
            .await?;
        Ok(key_id)
    }

    async fn get_ephemeral_secret(
        &self,
        key_id: &KeyId,
        description: &str,
    ) -> Result<StoredSecret> {
        let stored_secret = self.ephemeral_secrets.get(key_id).await?.ok_or_else(|| {
            VaultError::EntryNotFound(format!("{description} not found for key_id: '{key_id}'"))
        })?;
        Ok(stored_secret)
    }

    /// Remove secret from in memory storage
    async fn delete_ephemeral_secret(&self, key_id: KeyId) -> Result<bool> {
        self.ephemeral_secrets
            .delete(&key_id.clone())
            .await
            .map(|r| r.is_some())
    }
}

#[async_trait]
impl PersistentSecretsStore for VaultSecretsStore {
    async fn create_persistent_secret(&self, attributes: SecretAttributes) -> Result<KeyId> {
        self.security_module.create_secret(attributes).await
    }

    /// Remove secret from in memory storage
    async fn delete_persistent_secret(&self, key_id: KeyId) -> Result<bool> {
        self.security_module.delete_secret(key_id.clone()).await
    }
}

#[async_trait]
impl SecretsStoreReader for VaultSecretsStore {
    /// Get the secret attributes for a given key id
    async fn get_secret_attributes(&self, key_id: &KeyId) -> Result<SecretAttributes> {
        // search in the ephemeral secrets first, otherwise export the public key from the Kms
        if let Some(stored_secret) = self.ephemeral_secrets.get(key_id).await? {
            Ok(stored_secret.attributes())
        } else {
            self.security_module.get_attributes(key_id).await
        }
    }

    /// Extract public key a from secret
    async fn get_public_key(&self, key_id: &KeyId) -> Result<PublicKey> {
        // search in the ephemeral secrets first, otherwise export the public key from the Kms
        if let Some(stored_secret) = self.ephemeral_secrets.get(key_id).await? {
            VaultSecurityModule::compute_public_key_from_secret(stored_secret)
        } else {
            self.security_module.get_public_key(key_id).await
        }
    }

    async fn get_key_id(&self, public_key: &PublicKey) -> Result<KeyId> {
        self.security_module.get_key_id(public_key).await
    }
}

#[async_trait]
impl SecurityModule for VaultSecretsStore {
    async fn create_secret(&self, attributes: SecretAttributes) -> Result<KeyId> {
        self.security_module.create_secret(attributes).await
    }

    async fn get_public_key(&self, key_id: &KeyId) -> Result<PublicKey> {
        self.security_module.get_public_key(key_id).await
    }

    async fn get_key_id(&self, public_key: &PublicKey) -> Result<KeyId> {
        self.security_module.get_key_id(public_key).await
    }

    async fn get_attributes(&self, key_id: &KeyId) -> Result<SecretAttributes> {
        self.security_module.get_attributes(key_id).await
    }

    async fn delete_secret(&self, key_id: KeyId) -> Result<bool> {
        self.security_module.delete_secret(key_id).await
    }

    async fn sign(&self, key_id: &KeyId, message: &[u8]) -> Result<Signature> {
        self.security_module.sign(key_id, message).await
    }

    async fn verify(
        &self,
        public_key: &PublicKey,
        message: &[u8],
        signature: &Signature,
    ) -> Result<bool> {
        self.security_module
            .verify(public_key, message, signature)
            .await
    }
}

#[cfg(feature = "vault_tests")]
#[cfg(test)]
mod tests {
    use crate as ockam_vault;
    use crate::Vault;

    fn new_vault() -> Vault {
        Vault::new()
    }

    #[ockam_macros::vault_test]
    async fn test_create_ephemeral_secrets(vault: &mut impl SecretsStore) {}

    #[ockam_macros::vault_test]
    async fn test_secret_import_export(vault: &mut impl SecretsStore) {}

    #[ockam_macros::vault_test]
    async fn test_get_secret_attributes(vault: &mut impl SecretsStore) {}

    #[ockam_macros::vault_test]
    pub async fn test_get_key_id_by_public_key(vault: &mut impl SecretsStore) {}

    #[ockam_macros::vault_test]
    async fn test_get_key_id_for_persistent_secret_from_public_key(vault: &mut impl SecretsStore) {}
}
