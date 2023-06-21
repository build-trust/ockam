use crate::KeyValueStorage;
use ockam_core::compat::collections::BTreeMap;
use ockam_core::compat::{boxed::Box, sync::Arc, sync::RwLock, vec::Vec};
use ockam_core::{async_trait, Result};

/// In memory implementation of a key / value storage
#[derive(Clone)]
pub struct InMemoryKeyValueStorage<K, V> {
    storage: Arc<RwLock<BTreeMap<K, V>>>,
}

impl<K, V> Default for InMemoryKeyValueStorage<K, V> {
    fn default() -> Self {
        InMemoryKeyValueStorage {
            storage: Default::default(),
        }
    }
}

#[async_trait]
impl<K: Ord + Clone + Send + Sync + 'static, V: Clone + Send + Sync + 'static> KeyValueStorage<K, V>
    for InMemoryKeyValueStorage<K, V>
{
    async fn put(&self, key: K, value: V) -> Result<()> {
        let mut storage = self.storage.write().unwrap();
        storage.insert(key, value);
        Ok(())
    }

    async fn get(&self, key: &K) -> Result<Option<V>> {
        let storage = self.storage.read().unwrap();
        let value = storage.get(key).cloned();
        Ok(value)
    }

    async fn delete(&self, key: &K) -> Result<Option<V>> {
        let mut storage = self.storage.write().unwrap();
        Ok(storage.remove(key))
    }

    async fn keys(&self) -> Result<Vec<K>> {
        let storage = self.storage.read().unwrap();
        Ok(storage.keys().cloned().collect())
    }
}

impl<K: Ord + Clone + Sync + Send + 'static, V: Clone + Send + Sync + 'static>
    InMemoryKeyValueStorage<K, V>
{
    /// Create a new in-memory key / value storage
    pub fn new() -> InMemoryKeyValueStorage<K, V> {
        InMemoryKeyValueStorage {
            storage: Default::default(),
        }
    }
    /// Create a new in-memory key / value storage
    pub fn create() -> Arc<dyn KeyValueStorage<K, V>> {
        Arc::new(InMemoryKeyValueStorage::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ockam_core::Result;
    use serde::{Deserialize, Serialize};

    #[tokio::test]
    async fn test_key_value_storage() -> Result<()> {
        let storage = InMemoryKeyValueStorage::<u8, Value>::create();

        // a value can be inserted
        storage.put(1, Value(10)).await.unwrap();

        // the new value can be retrieved by key
        let retrieved = storage.get(&0).await?;
        assert_eq!(retrieved, None);

        let retrieved = storage.get(&1).await?;
        assert_eq!(retrieved, Some(Value(10)));

        Ok(())
    }

    #[derive(Serialize, Deserialize, Default, PartialEq, Eq, Debug, Clone)]
    struct Value(u8);
}
