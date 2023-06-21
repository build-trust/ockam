use ockam_core::compat::{boxed::Box, vec::Vec};
use ockam_core::{async_trait, Result};

/// This trait defines a key/value storage
#[async_trait]
pub trait KeyValueStorage<K, V>: Sync + Send + 'static {
    /// Store a key / value
    async fn put(&self, key: K, value: V) -> Result<()>;

    /// Retrieve a value. Return None if no value corresponds to the specified key
    async fn get(&self, key: &K) -> Result<Option<V>>;

    /// Delete a value and return it if found
    async fn delete(&self, key: &K) -> Result<Option<V>>;

    /// Return the list of all the keys
    async fn keys(&self) -> Result<Vec<K>>;
}
