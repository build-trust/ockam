use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::ToStringKey;
use ockam_core::compat::boxed::Box;
use ockam_core::compat::collections::BTreeMap;
use ockam_core::compat::string::String;
use ockam_core::{async_trait, Result};

use crate::{FileValueStorage, InMemoryKeyValueStorage, KeyValueStorage, ValueStorage};

/// Key value storage backed by a file
/// An additional cache in used to access values in memory and avoid re-reading the file too
/// frequently
/// WARNING: This implementation provides limited consistency if the same file is reused from
/// multiple instances and/or processes. For example, if one process deletes a value, the other
/// process will still have it in its cache and return it on a Get query.
/// NOTE: Currently, unused
pub struct FileKeyValueStorage<K, V> {
    file_storage: FileValueStorage<BTreeMap<String, V>>,
    cache: InMemoryKeyValueStorage<K, V>,
}

impl<
        K: Serialize + for<'de> Deserialize<'de> + ToStringKey + Ord + Clone + Send + Sync + 'static,
        V: Default + Serialize + for<'de> Deserialize<'de> + Clone + Send + Sync + 'static,
    > FileKeyValueStorage<K, V>
{
    /// Create the file storage and in memory cache
    pub async fn create(path: &Path) -> Result<Self> {
        Ok(Self {
            file_storage: FileValueStorage::create(path).await?,
            cache: InMemoryKeyValueStorage::new(),
        })
    }
}

#[async_trait]
impl<
        K: Serialize + for<'de> Deserialize<'de> + ToStringKey + Ord + Clone + Send + Sync + 'static,
        V: Clone + Serialize + for<'de> Deserialize<'de> + Send + Sync + 'static,
    > KeyValueStorage<K, V> for FileKeyValueStorage<K, V>
{
    /// Put a value in the file storage and in cache for faster access
    async fn put(&self, key: K, value: V) -> Result<()> {
        let (k, v) = (key.clone(), value.clone());
        let f = move |mut map: BTreeMap<String, V>| {
            map.insert(key.to_string_key(), value.clone());
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
            let k = key.to_string_key();
            let f = move |map: BTreeMap<String, V>| Ok(map.get(&k).cloned());
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
        let k = key.to_string_key();
        let f = move |mut map: BTreeMap<String, V>| {
            let removed = map.remove(&k);
            Ok((map, removed))
        };
        self.file_storage.modify_value(f).await?;
        self.cache.delete(key).await
    }

    /// Return the list of all the keys **in cache**
    async fn keys(&self) -> Result<Vec<K>> {
        self.cache.keys().await
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    use ockam_core::Result;

    #[tokio::test]
    async fn test_file_key_value_storage() -> Result<()> {
        let file = NamedTempFile::new().unwrap();
        let storage = FileKeyValueStorage::<Key, Value>::create(file.path()).await?;

        // persist a new value
        storage.put(Key::new(1, 2), Value(10)).await.unwrap();

        // retrieve the value
        let missing = storage.get(&Key::new(0, 0)).await?;
        assert_eq!(missing, None);

        let updated = storage.get(&Key::new(1, 2)).await?;
        assert_eq!(updated, Some(Value(10)));

        Ok(())
    }

    #[derive(Serialize, Deserialize, Default, PartialEq, Eq, Clone, Debug, PartialOrd, Ord)]
    struct Value(u8);

    #[derive(Serialize, Deserialize, Default, PartialEq, Eq, Clone, Debug, PartialOrd, Ord)]
    struct Key {
        key1: u8,
        key2: u8,
    }

    impl ToStringKey for Key {
        fn to_string_key(&self) -> String {
            format!("{}_{}", self.key1, self.key2)
        }
    }

    impl Key {
        fn new(key1: u8, key2: u8) -> Self {
            Self { key1, key2 }
        }
    }
}
