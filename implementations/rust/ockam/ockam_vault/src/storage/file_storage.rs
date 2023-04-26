use crate::{KeyId, Secret, SecretAttributes, SecretPersistence, VaultEntry};
use ockam_core::compat::boxed::Box;
use ockam_core::compat::sync::Arc;
use ockam_core::{async_trait, Result};
use ockam_node::{FileValueStorage, KeyValueStorage, ValueStorage};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Storage for a Vault backed by a file
pub struct VaultFileStorage {
    storage: Arc<FileValueStorage<FileVault>>,
}

impl VaultFileStorage {
    /// Create a new file storage for a Vault
    pub async fn create(path: &Path) -> Result<Arc<dyn KeyValueStorage<KeyId, VaultEntry>>> {
        let storage = Arc::new(FileValueStorage::create(path).await?);
        Ok(Arc::new(VaultFileStorage { storage }))
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct FileVaultEntry {
    key_id: Option<String>,
    key_attributes: SecretAttributes,
    key: Secret,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "version")]
#[non_exhaustive]
enum FileVault {
    V1 {
        entries: Vec<(usize, FileVaultEntry)>,
        next_id: usize,
    },
}

impl Default for FileVault {
    fn default() -> Self {
        FileVault::V1 {
            entries: Default::default(),
            next_id: Default::default(),
        }
    }
}

#[async_trait]
impl KeyValueStorage<KeyId, VaultEntry> for VaultFileStorage {
    async fn put(&self, key_id: KeyId, key: VaultEntry) -> Result<()> {
        let attributes = key.key_attributes();
        let key = key.secret().clone();
        let t = move |v: FileVault| {
            let new_entry = (
                0,
                FileVaultEntry {
                    key_id: Some(key_id.clone()),
                    key_attributes: attributes.clone(),
                    key: key.clone(),
                },
            );
            let FileVault::V1 {
                mut entries,
                next_id,
            } = v;
            entries.push(new_entry);
            Ok(FileVault::V1 { entries, next_id })
        };
        self.storage.update_value(t).await
    }

    async fn get(&self, key_id: &KeyId) -> Result<Option<VaultEntry>> {
        let key_id = key_id.clone();
        let t = move |v: FileVault| -> Result<Option<VaultEntry>> {
            let FileVault::V1 {
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
                .map(|le| VaultEntry::new(le.1.key_attributes, le.1.key.clone())))
        };
        self.storage.read_value(t).await
    }

    async fn delete(&self, key_id: &KeyId) -> Result<Option<VaultEntry>> {
        let key_id = key_id.clone();
        let t = move |v: FileVault| -> Result<(FileVault, Option<VaultEntry>)> {
            let FileVault::V1 {
                mut entries,
                next_id,
            } = v.clone();
            if let Some(index) = entries.iter_mut().position(|(_, entry)| {
                if let Some(id) = &entry.key_id {
                    id.eq(&key_id)
                } else {
                    false
                }
            }) {
                let removed = entries.swap_remove(index);
                let vault_entry = VaultEntry::new(removed.1.key_attributes, removed.1.key);
                Ok((FileVault::V1 { entries, next_id }, Some(vault_entry)))
            } else {
                Ok((v, None))
            }
        };
        self.storage.modify_value(t).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SecretKey, SecretType, SecretVault, Vault};
    use ockam_core::compat::join;
    use ockam_core::compat::rand::RngCore;
    use rand::thread_rng;
    use std::path::PathBuf;

    #[test]
    fn test_parse_legacy_key() {
        //it's easier to embed a json formatted as base64 rather than a literal string
        let sample_key =
            "eyJ2ZXJzaW9uIjoiVjEiLCJlbnRyaWVzIjpbWzAseyJrZXlfaWQiOiI1N2ZjOGI3OGNlMzg4OWM1\
MWMwMzYyYzllZjk1NDU0ZjFiYjFkYjgwYmM3Y2I3ZDZlOGQzZGVjNTIxNGVkYzRkIiwia2V5X2F0\
dHJpYnV0ZXMiOnsic3R5cGUiOiJFZDI1NTE5IiwicGVyc2lzdGVuY2UiOiJQZXJzaXN0ZW50Iiwi\
bGVuZ3RoIjozMn0sImtleSI6eyJLZXkiOlsyMywyNTEsMzQsMTE2LDEyMSwxMjQsODUsMTEsMjUz\
LDc1LDEyOSwxMDksODgsMjM1LDE4OSw4OCwyMjYsMTUwLDQzLDU1LDE4NywxNDksMjQ3LDEzNywx\
NjMsMTY2LDEzMSw0NCwxMjYsMTMzLDIyOSwxMzldfX1dXSwibmV4dF9pZCI6MH0=";

        let text = String::from_utf8(data_encoding::BASE64.decode(sample_key.as_bytes()).unwrap())
            .unwrap();

        let vault: FileVault = serde_json::from_str(&text).unwrap();

        #[allow(irrefutable_let_patterns)] //can be removed when we'll have V2
        let (entries, next_id) = if let FileVault::V1 { entries, next_id } = vault {
            (entries, next_id)
        } else {
            panic!("legacy deserialization is broken")
        };

        assert_eq!(0, next_id);
        assert_eq!(1, entries.len());

        let (id, entry) = entries.get(0).unwrap();
        assert_eq!(&0, id);

        assert_eq!(
            "57fc8b78ce3889c51c0362c9ef95454f1bb1db80bc7cb7d6e8d3dec5214edc4d",
            entry.key_id.as_ref().unwrap()
        );
        assert_eq!(SecretType::Ed25519, entry.key_attributes.stype());
        assert_eq!(
            SecretPersistence::Persistent,
            entry.key_attributes.persistence()
        );
        assert_eq!(32, entry.key_attributes.length());
        let secret_key = SecretKey::new(vec![
            23, 251, 34, 116, 121, 124, 85, 11, 253, 75, 129, 109, 88, 235, 189, 88, 226, 150, 43,
            55, 187, 149, 247, 137, 163, 166, 131, 44, 126, 133, 229, 139,
        ]);
        assert_eq!(&secret_key, entry.key.cast_as_key());
    }

    #[tokio::test]
    #[allow(non_snake_case)]
    async fn test_secret_persistence__recreate_vault__loads_from_storage() {
        let storage = VaultFileStorage::create(create_temp_file().as_path())
            .await
            .unwrap();
        let vault = Vault::new(storage.clone());

        let attributes10 =
            SecretAttributes::new(SecretType::Ed25519, SecretPersistence::Persistent, 0);
        let attributes20 =
            SecretAttributes::new(SecretType::X25519, SecretPersistence::Persistent, 0);
        let attributes3 =
            SecretAttributes::new(SecretType::X25519, SecretPersistence::Ephemeral, 0);

        let key_id1 = vault.secret_generate(attributes10).await.unwrap();
        let key_id2 = vault.secret_generate(attributes20).await.unwrap();
        let key_id3 = vault.secret_generate(attributes3).await.unwrap();

        let vault = Vault::new(storage.clone());

        let attributes11 = vault.secret_attributes_get(&key_id1).await.unwrap();
        assert_eq!(attributes10, attributes11);
        let attributes21 = vault.secret_attributes_get(&key_id2).await.unwrap();
        assert_eq!(attributes20, attributes21);
        let attributes31 = vault.secret_attributes_get(&key_id3).await;
        assert!(attributes31.is_err());
    }

    #[tokio::test]
    async fn test_vault_synchronization() {
        let storage = VaultFileStorage::create(create_temp_file().as_path())
            .await
            .unwrap();
        let vault = Vault::new(storage);

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

    pub fn create_temp_file() -> PathBuf {
        let dir = std::env::temp_dir();
        let mut rng = thread_rng();
        let mut bytes = [0u8; 32];
        rng.fill_bytes(&mut bytes);
        let file_name = hex::encode(bytes);
        dir.join(file_name)
    }
}
