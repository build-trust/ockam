use crate::storage::SecretStorage;
use crate::{KeyId, Vault, VaultEntry, VaultError};
use ockam_core::{async_trait, Result};
use ockam_node::KeyValueStorage;

/// Implementation the SecretStorage trait for a Vault using the 2 storages attached to a Vault:
/// persistent and ephemeral
#[async_trait]
impl SecretStorage for Vault {
    async fn store_secret(&self, key_id: &KeyId, key: &VaultEntry) -> Result<()> {
        if key.key_attributes().is_persistent() {
            self.persistent_storage
                .put(key_id.clone(), key.clone())
                .await
        } else {
            self.ephemeral_storage
                .put(key_id.clone(), key.clone())
                .await
        }
    }

    /// Return a key from storage. First try the ephemeral storage, then the persistent one
    /// The key is expected to be found, otherwise an error is returned
    async fn get_secret(&self, secret: &KeyId, description: &str) -> Result<VaultEntry> {
        if let Ok(vault_entry) = self.get_ephemeral_secret(secret, description).await {
            Ok(vault_entry)
        } else {
            self.get_persistent_secret(secret, description).await
        }
    }

    /// The key is expected to be found, otherwise an error is returned
    async fn get_ephemeral_secret(&self, secret: &KeyId, description: &str) -> Result<VaultEntry> {
        self.ephemeral_storage.get(secret).await?.ok_or_else(|| {
            VaultError::EntryNotFound(format!("missing {description} for {secret:?}")).into()
        })
    }

    /// Return a key from persistent storage
    /// The key is expected to be found, otherwise an error is returned
    async fn get_persistent_secret(&self, secret: &KeyId, description: &str) -> Result<VaultEntry> {
        self.persistent_storage.get(secret).await?.ok_or_else(|| {
            VaultError::EntryNotFound(format!("missing {description} for {secret:?}")).into()
        })
    }

    async fn delete_secret(&self, key_id: &KeyId) -> Result<Option<VaultEntry>> {
        if let Some(persisted) = self.persistent_storage.delete(key_id).await? {
            Ok(Some(persisted))
        } else {
            self.ephemeral_storage.delete(key_id).await
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::storage::SecretStorage;
    use crate::{
        KeyId, Secret, SecretAttributes, SecretKey, SecretPersistence, SecretType, Vault,
        VaultEntry,
    };
    use ockam_core::Result;

    #[tokio::test]
    async fn test_vault_store() -> Result<()> {
        let vault = Vault::create();

        // a persistent secret is only available as a persistent vault entry
        let vault_entry = create_vault_entry(SecretPersistence::Persistent);
        let key_id: KeyId = "key-1".into();
        vault.store_secret(&key_id, &vault_entry).await?;
        let actual = vault.get_ephemeral_secret(&key_id, "entry").await.ok();
        assert_eq!(actual, None);

        let actual = vault.get_persistent_secret(&key_id, "entry").await.ok();
        assert_eq!(actual, Some(vault_entry));

        let vault_entry = create_vault_entry(SecretPersistence::Ephemeral);
        let key_id: KeyId = "key-2".into();
        vault.store_secret(&key_id, &vault_entry).await?;

        let actual = vault.get_ephemeral_secret(&key_id, "entry").await.ok();
        assert_eq!(actual, Some(vault_entry));

        let actual = vault.get_persistent_secret(&key_id, "entry").await.ok();
        assert_eq!(actual, None);

        Ok(())
    }

    fn create_vault_entry(persistence: SecretPersistence) -> VaultEntry {
        let vault_entry = VaultEntry::new(
            SecretAttributes::new(SecretType::Ed25519, persistence, 1),
            Secret::Key(SecretKey::new(vec![1])),
        );
        vault_entry
    }
}
