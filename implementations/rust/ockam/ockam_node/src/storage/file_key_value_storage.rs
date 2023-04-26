use crate::{FileValueStorage, InMemoryKeyValueStorage, KeyValueStorage, ValueStorage};
use ockam_core::compat::boxed::Box;
use ockam_core::compat::collections::BTreeMap;
use ockam_core::{async_trait, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Key value storage backed by a file
/// An additional cache in used to access values in memory and avoid re-reading the file too
/// frequently
pub struct FileKeyValueStorage<K, V> {
    file_storage: FileValueStorage<BTreeMap<K, V>>,
    cache: InMemoryKeyValueStorage<K, V>,
}

impl<
        K: Ord + Serialize + for<'de> Deserialize<'de>,
        V: Default + Serialize + for<'de> Deserialize<'de>,
    > FileKeyValueStorage<K, V>
{
    /// Create the file storage and in memory cache
    pub async fn create(path: &Path) -> Result<Self> {
        Ok(Self {
            file_storage: FileValueStorage::create(path).await?,
            cache: InMemoryKeyValueStorage::create(),
        })
    }
}

#[async_trait]
impl<
        K: Clone + Ord + Serialize + for<'de> Deserialize<'de> + Send + Sync + 'static,
        V: Clone + Serialize + for<'de> Deserialize<'de> + Send + Sync + 'static,
    > KeyValueStorage<K, V> for FileKeyValueStorage<K, V>
{
    /// Put a value in the file storage and in cache for faster access
    async fn put(&self, key: K, value: V) -> Result<()> {
        let (k, v) = (key.clone(), value.clone());
        let f = move |mut map: BTreeMap<K, V>| {
            map.insert(key.clone(), value.clone());
            Ok(map)
        };
        self.file_storage.update_value(f).await?;
        self.cache.put(k, v).await
    }

    /// Get a value from cache.
    /// If the value is not found in the cache try to find it in the file, then cache it
    async fn get(&self, key: &K) -> Result<Option<V>> {
        if let Some(value) = self.cache.get(key).await? {
            Ok(Some(value))
        } else {
            let k = key.clone();
            let f = move |map: BTreeMap<K, V>| Ok(map.get(&k).map(|v| v.clone()));
            let retrieved_value = self.file_storage.read_value(f).await?;
            if let Some(retrieved) = retrieved_value.clone() {
                self.cache.put(key.clone(), retrieved).await?;
            }
            Ok(retrieved_value)
        }
    }

    /// Delete a value from the file and the cache
    /// Return the value if it was found
    async fn delete(&self, key: &K) -> Result<Option<V>> {
        let k = key.clone();
        let f = move |mut map: BTreeMap<K, V>| {
            let removed = map.remove(&k);
            Ok((map, removed))
        };
        self.file_storage.modify_value(f).await?;
        self.cache.delete(key).await
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use ockam_core::compat::rand::{thread_rng, RngCore};
    use ockam_core::Result;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_file_key_value_storage() -> Result<()> {
        let storage: FileKeyValueStorage<u8, Value> =
            FileKeyValueStorage::create(create_temp_file().as_path())
                .await
                .unwrap();

        // persist a new value
        storage.put(1, Value(10)).await.unwrap();

        // retrieve the value
        let missing = storage.get(&0).await?;
        assert_eq!(missing, None);

        let updated = storage.get(&1).await?;
        assert_eq!(updated, Some(Value(10)));

        Ok(())
    }

    pub fn create_temp_file() -> PathBuf {
        let dir = std::env::temp_dir();
        let mut rng = thread_rng();
        let mut bytes = [0u8; 32];
        rng.fill_bytes(&mut bytes);
        let file_name = hex::encode(bytes);
        dir.join(file_name)
    }

    #[derive(Serialize, Deserialize, Default, PartialEq, Eq, Clone, Debug)]
    struct Value(u8);
}
