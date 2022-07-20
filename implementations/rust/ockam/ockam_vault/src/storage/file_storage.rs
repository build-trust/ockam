use crate::VaultError;
use ockam_core::compat::boxed::Box;
use ockam_core::vault::storage::Storage;
use ockam_core::vault::{KeyId, SecretAttributes, SecretKey, SecretPersistence, VaultEntry};
use ockam_core::{async_trait, Result};
use ockam_node::compat::asynchronous::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use tracing::debug;

#[derive(Serialize, Deserialize)]
struct LegacyVaultEntry {
    key_id: Option<String>,
    key_attributes: SecretAttributes,
    key: SecretKey,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "version")]
#[non_exhaustive]
enum LegacySerializedVault {
    V1 {
        entries: Vec<(usize, LegacyVaultEntry)>,
        next_id: usize,
    },
}

type Data = RwLock<BTreeMap<KeyId, VaultEntry>>;

/// File Storage
pub struct FileStorage {
    path: PathBuf,
    temp_path: PathBuf,
    data: Data,
}

impl FileStorage {
    async fn deserialize(vault_bytes: &[u8]) -> Result<Data> {
        let vault: LegacySerializedVault =
            serde_json::from_slice(vault_bytes).map_err(|_| VaultError::InvalidStorageData)?;

        let data = Data::default();

        match vault {
            LegacySerializedVault::V1 { entries, .. } => {
                let mut data_lock = data.write().await;
                for entry in entries {
                    let entry = entry.1;
                    if entry.key_attributes.persistence() != SecretPersistence::Persistent {
                        continue;
                    }

                    let key_id = match entry.key_id {
                        Some(key_id) => key_id,
                        None => continue,
                    };

                    data_lock.insert(key_id, VaultEntry::new(entry.key_attributes, entry.key));
                }
            }
        }

        Ok(data)
    }

    async fn serialize(&self) -> Result<Vec<u8>> {
        let entries = self.data.read().await;

        let legacy_entries = entries
            .iter()
            .map(|(k, e)| {
                (
                    0,
                    LegacyVaultEntry {
                        key_id: Some(k.clone()),
                        key_attributes: e.key_attributes(),
                        key: e.key().clone(),
                    },
                )
            })
            .collect();

        let v = LegacySerializedVault::V1 {
            entries: legacy_entries,
            next_id: 0,
        };

        serde_json::to_vec(&v).map_err(|_| VaultError::StorageError.into())
    }

    fn get_temp_path(path: &Path) -> PathBuf {
        let tmp_ext = match path.extension() {
            None => ".tmp".to_string(),
            Some(e) => format!("{}.tmp", e.to_str().unwrap()),
        };

        path.with_extension(tmp_ext)
    }

    async fn flush_to_file(&self) -> Result<()> {
        let data = self.serialize().await?;

        use std::io::prelude::*;
        use std::os::unix::prelude::*;

        let _ = std::fs::remove_file(&self.temp_path);

        let mut file = std::fs::OpenOptions::new()
            .write(true)
            // `create_new` means we error if it exists. This ensures the mode we
            // provide is respect (the `mode(0o600)` is only used if creating the
            // file)
            .create_new(true)
            .mode(0o600) // TODO: not portable, what about windows?
            .open(&self.temp_path)
            .map_err(|_| VaultError::StorageError)?;
        file.write_all(&data)
            .map_err(|_| VaultError::StorageError)?;
        file.flush().map_err(|_| VaultError::StorageError)?;
        file.sync_all().map_err(|_| VaultError::StorageError)?;

        std::fs::rename(&self.temp_path, &self.path).map_err(|_| VaultError::StorageError)?;

        Ok(())
    }

    /// Create FileStorage using file at given Path
    /// If file doesn't exist, it will be created
    pub async fn init(&mut self) -> Result<()> {
        self.data = if !self.path.exists() {
            Default::default()
        } else {
            let vault_bytes = std::fs::read(&self.path).map_err(|_| VaultError::StorageError)?;
            Self::deserialize(&vault_bytes).await?
        };

        let _ = std::fs::remove_file(&self.temp_path);

        self.flush_to_file().await?;

        Ok(())
    }

    /// Constructor.
    /// NOTE: Doesn't initialize the storage. Call [`FileStorage::init()`] or use [`FileStorage::create()`]
    pub fn new(path: PathBuf) -> Self {
        let tmp_path = Self::get_temp_path(&path);

        Self {
            path,
            temp_path: tmp_path,
            data: Default::default(),
        }
    }

    /// Create and init Storage
    pub async fn create(path: PathBuf) -> Result<Self> {
        let mut s = Self::new(path);
        s.init().await?;

        Ok(s)
    }

    /// Clear the Storage
    pub async fn clear(&self) {
        if self.path.exists() {
            debug!("Note: removing previous file at {:?}", &self.path);
            let _ = std::fs::remove_file(&self.path);
        }

        let _ = std::fs::remove_file(&self.temp_path);
    }
}

#[async_trait]
impl Storage for FileStorage {
    async fn store(&self, key_id: &KeyId, key: &VaultEntry) -> Result<()> {
        let _ = self.data.write().await.insert(key_id.clone(), key.clone());

        self.flush_to_file().await?;

        Ok(())
    }

    async fn load(&self, key_id: &KeyId) -> Result<VaultEntry> {
        Ok(self
            .data
            .read()
            .await
            .get(key_id)
            .ok_or(VaultError::EntryNotFound)?
            .clone())
    }

    async fn delete(&self, key_id: &KeyId) -> Result<VaultEntry> {
        let entry = self
            .data
            .write()
            .await
            .remove(key_id)
            .ok_or(VaultError::EntryNotFound)?;

        self.flush_to_file().await?;

        Ok(entry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Vault;
    use ockam_core::compat::rand::RngCore;
    use ockam_core::vault::{SecretType, SecretVault};
    use rand::thread_rng;
    use std::sync::Arc;

    #[tokio::test]
    #[allow(non_snake_case)]
    async fn secret_persistence__recreate_vault__loads_from_storage() {
        let mut rng = thread_rng();
        let mut rand_id = [0u8; 32];

        rng.fill_bytes(&mut rand_id);
        let rand_id1 = hex::encode(&rand_id);

        rng.fill_bytes(&mut rand_id);

        let dir = std::env::temp_dir();
        let storage = FileStorage::create(dir.join(rand_id1)).await.unwrap();
        let storage = Arc::new(storage);
        let vault = Vault::new(Some(storage.clone()));

        let attributes10 =
            SecretAttributes::new(SecretType::Ed25519, SecretPersistence::Persistent, 0);
        let attributes20 =
            SecretAttributes::new(SecretType::X25519, SecretPersistence::Persistent, 0);
        let attributes3 =
            SecretAttributes::new(SecretType::X25519, SecretPersistence::Ephemeral, 0);

        let key_id1 = vault.secret_generate(attributes10).await.unwrap();
        let key_id2 = vault.secret_generate(attributes20).await.unwrap();
        let key_id3 = vault.secret_generate(attributes3).await.unwrap();

        let vault = Vault::new(Some(storage.clone()));

        let attributes11 = vault.secret_attributes_get(&key_id1).await.unwrap();
        assert_eq!(attributes10, attributes11);
        let attributes21 = vault.secret_attributes_get(&key_id2).await.unwrap();
        assert_eq!(attributes20, attributes21);
        let attributes31 = vault.secret_attributes_get(&key_id3).await;
        assert!(attributes31.is_err());
    }
}
