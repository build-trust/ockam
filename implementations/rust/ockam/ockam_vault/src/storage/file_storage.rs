use crate::VaultError;
use cfg_if::cfg_if;
use fs2::FileExt; //locking
use ockam_core::compat::boxed::Box;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::vault::storage::Storage;
use ockam_core::vault::{KeyId, Secret, SecretAttributes, SecretPersistence, VaultEntry};
use ockam_core::{async_trait, Error, Result};
use ockam_node::tokio::task::{self, JoinError};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Debug)]
struct LegacyVaultEntry {
    key_id: Option<String>,
    key_attributes: SecretAttributes,
    key: Secret,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "version")]
#[non_exhaustive]
enum LegacySerializedVault {
    V1 {
        entries: Vec<(usize, LegacyVaultEntry)>,
        next_id: usize,
    },
}

/// File Storage
/* There are three files involved
 * - The actual vault file
 * - A temp file used to avoid data lost during writtes:  vault is entirely
 *   written to the temp file, then file renamed.
 * - A "lock" file.  It's used to control inter-process access to the vault.
 *   Before reading or writting to the vault, first need to get a shared or exclusive lock
 *   on this file.  We don't lock over the vault file directly, because doesn't play well with
 *   the file rename we do */
pub struct FileStorage {
    path: PathBuf,
    temp_path: PathBuf,
    lock_path: PathBuf,
}

fn map_join_err(err: JoinError) -> Error {
    Error::new(Origin::Application, Kind::Io, err)
}
fn map_io_err(err: std::io::Error) -> Error {
    Error::new(Origin::Application, Kind::Io, err)
}

impl FileStorage {
    /// Create FileStorage using file at given Path
    /// If file doesn't exist, it will be created
    pub async fn init(&mut self) -> Result<()> {
        // This can block, but only when first initializing and just need to write an empty vault.
        // So didn't bother to do it async
        let lock_file = Self::open_lock_file(&self.lock_path)?;
        lock_file.lock_exclusive().map_err(map_io_err)?;
        if !self.path.exists() {
            let empty = LegacySerializedVault::V1 {
                entries: Vec::new(),
                next_id: 0,
            };
            Self::flush_to_file(&self.path, &self.temp_path, &empty)?;
        }
        lock_file.unlock().map_err(map_io_err)?;
        Ok(())
    }

    fn path_with_suffix(path: &Path, suffix: &str) -> PathBuf {
        match path.extension() {
            None => path.with_extension(suffix),
            Some(e) => path.with_extension(format!("{}{}", e.to_str().unwrap(), suffix)),
        }
    }

    fn load(path: &PathBuf) -> Result<LegacySerializedVault> {
        let file = File::open(path).map_err(map_io_err)?;
        let reader = BufReader::new(file);
        Ok(serde_json::from_reader(reader).map_err(|_| VaultError::InvalidStorageData)?)
    }

    fn open_lock_file(lock_path: &PathBuf) -> Result<File> {
        std::fs::OpenOptions::new()
            .write(true)
            .read(true)
            .create(true)
            .open(lock_path)
            .map_err(map_io_err)
    }

    /// Constructor.
    /// NOTE: Doesn't initialize the storage. Call [`FileStorage::init()`] or use [`FileStorage::create()`]
    pub fn new(path: PathBuf) -> Self {
        let temp_path = Self::path_with_suffix(&path, ".tmp");
        let lock_path = Self::path_with_suffix(&path, ".lock");
        Self {
            path,
            temp_path,
            lock_path,
        }
    }

    /// Create and init Storage
    pub async fn create(path: PathBuf) -> Result<Self> {
        let mut s = Self::new(path);
        s.init().await?;

        Ok(s)
    }

    // Flush vault to target, using temp_path as intermediary file.
    fn flush_to_file(
        target: &PathBuf,
        temp_path: &PathBuf,
        vault: &LegacySerializedVault,
    ) -> Result<()> {
        let data = serde_json::to_vec(vault).map_err(|_| VaultError::StorageError)?;
        use std::io::prelude::*;

        cfg_if! {
            if #[cfg(windows)] {
                let mut file = std::fs::OpenOptions::new()
                    .write(true)
                    .create(true)
                    .open(temp_path)
                    .map_err(|_| VaultError::StorageError)?;
            } else {
                use std::os::unix::fs::OpenOptionsExt;
                let mut file = std::fs::OpenOptions::new()
                    .write(true)
                    .create(true)
                    .mode(0o600)
                    .open(temp_path)
                    .map_err(|_| VaultError::StorageError)?;
            }
        }
        file.write_all(&data)
            .map_err(|_| VaultError::StorageError)?;
        file.flush().map_err(|_| VaultError::StorageError)?;
        file.sync_all().map_err(|_| VaultError::StorageError)?;
        std::fs::rename(temp_path, target).map_err(|_| VaultError::StorageError)?;
        Ok(())
    }

    async fn write_transaction<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(LegacySerializedVault) -> Result<(LegacySerializedVault, R)> + Send + 'static,
        R: Send + 'static,
    {
        let lock_path = self.lock_path.clone();
        let temp_path = self.temp_path.clone();
        let path = self.path.clone();
        let tr = move || -> Result<R> {
            let file = FileStorage::open_lock_file(&lock_path)?;
            file.lock_exclusive().map_err(map_io_err)?;
            let vault_data = FileStorage::load(&path)?;
            let (modified_vault, result) = f(vault_data)?;
            FileStorage::flush_to_file(&path, &temp_path, &modified_vault)?;
            // if something goes wrong it will be unlocked once the file handler get closed anyway
            file.unlock().map_err(map_io_err)?;
            Ok(result)
        };
        task::spawn_blocking(tr).await.map_err(map_join_err)?
    }

    async fn read_transaction<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(LegacySerializedVault) -> Result<R> + Send + 'static,
        R: Send + 'static,
    {
        let path = self.path.clone();
        let lock_path = self.lock_path.clone();
        let tr = move || {
            let file = FileStorage::open_lock_file(&lock_path)?;
            file.lock_shared().map_err(map_io_err)?;
            let vault_data = FileStorage::load(&path)?;
            let r = f(vault_data)?;
            // if something goes wrong it will be unlocked once the file handler get closed anyway
            file.unlock().map_err(map_io_err)?;
            Ok(r)
        };
        task::spawn_blocking(tr).await.map_err(map_join_err)?
    }
}

#[async_trait]
impl Storage for FileStorage {
    async fn store(&self, key_id: &KeyId, key: &VaultEntry) -> Result<()> {
        let key_id = key_id.clone();
        let attributes = key.key_attributes();
        let key = key.secret().clone();
        let t = move |v: LegacySerializedVault| {
            let new_entry = (
                0,
                LegacyVaultEntry {
                    key_id: Some(key_id),
                    key_attributes: attributes,
                    key,
                },
            );
            let LegacySerializedVault::V1 {
                mut entries,
                next_id,
            } = v;
            entries.push(new_entry);
            Ok((LegacySerializedVault::V1 { entries, next_id }, ()))
        };
        self.write_transaction(t).await
    }

    async fn load(&self, key_id: &KeyId) -> Result<VaultEntry> {
        let key_id = key_id.clone();
        let t = move |v: LegacySerializedVault| -> Result<VaultEntry> {
            let LegacySerializedVault::V1 {
                entries,
                next_id: _,
            } = v;
            Ok(entries
                .iter()
                .find(|x| {
                    if let Some(id) = &x.1.key_id {
                        id.eq(&key_id)
                            && x.1.key_attributes.persistence() == SecretPersistence::Persistent
                    } else {
                        false
                    }
                })
                .map(|le| VaultEntry::new(le.1.key_attributes, le.1.key.clone()))
                .ok_or(VaultError::EntryNotFound)?)
        };
        self.read_transaction(t).await
    }

    async fn delete(&self, key_id: &KeyId) -> Result<VaultEntry> {
        let key_id = key_id.clone();
        let t = move |v: LegacySerializedVault| -> Result<(LegacySerializedVault, VaultEntry)> {
            let LegacySerializedVault::V1 {
                mut entries,
                next_id,
            } = v;
            if let Some(index) = entries.iter_mut().position(|(_, entry)| {
                if let Some(id) = &entry.key_id {
                    id.eq(&key_id)
                } else {
                    false
                }
            }) {
                let removed = entries.swap_remove(index);
                let vault_entry = VaultEntry::new(removed.1.key_attributes, removed.1.key);
                Ok((LegacySerializedVault::V1 { entries, next_id }, vault_entry))
            } else {
                Err(Error::from(VaultError::EntryNotFound))
            }
        };
        self.write_transaction(t).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Vault;
    use ockam_core::compat::join;
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
        let rand_id1 = hex::encode(rand_id);

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

    #[tokio::test]
    #[allow(non_snake_case)]
    async fn vault_syncronization() {
        let mut rng = thread_rng();
        let mut rand_id = [0u8; 32];

        rng.fill_bytes(&mut rand_id);
        let rand_id1 = hex::encode(rand_id);

        rng.fill_bytes(&mut rand_id);

        let dir = std::env::temp_dir();
        let storage = FileStorage::create(dir.join(rand_id1)).await.unwrap();

        let storage = Arc::new(storage);
        let vault = Vault::new(Some(storage.clone()));

        let attributes1 =
            SecretAttributes::new(SecretType::Ed25519, SecretPersistence::Persistent, 0);
        let attributes2 =
            SecretAttributes::new(SecretType::Ed25519, SecretPersistence::Persistent, 0);
        let attributes3 =
            SecretAttributes::new(SecretType::Ed25519, SecretPersistence::Persistent, 0);

        let (key_id1, key_id2, key_id3) = join!(
            vault.secret_generate(attributes1),
            vault.secret_generate(attributes2),
            vault.secret_generate(attributes3)
        );

        let key_id1 = key_id1.unwrap();
        let key_id2 = key_id2.unwrap();
        let key_id3 = key_id3.unwrap();

        let (attributes12, attributes22, attributes32) = join!(
            vault.secret_attributes_get(&key_id1),
            vault.secret_attributes_get(&key_id2),
            vault.secret_attributes_get(&key_id3)
        );

        assert_eq!(attributes1, attributes12.unwrap());
        assert_eq!(attributes2, attributes22.unwrap());
        assert_eq!(attributes3, attributes32.unwrap());
    }
}
