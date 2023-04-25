use crate::ValueStorage;
use ockam_core::{async_trait, compat::boxed::Box, compat::sync::RwLock, Result};

/// In memory implementation of a value storage
pub struct InMemoryValueStorage<V> {
    storage: RwLock<V>,
}

#[async_trait]
impl<V: Send + Sync + Clone + 'static> ValueStorage<V> for InMemoryValueStorage<V> {
    async fn update_value<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(V) -> Result<(V, R)> + Send + 'static,
        R: Send + 'static,
    {
        let mut value = self.storage.write().unwrap().clone();
        let (updated, result) = f(value)?;
        value = updated;
        Ok(result)
    }

    async fn read_value<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(V) -> Result<R> + Send + 'static,
        R: Send + 'static,
    {
        f(self.storage.read().unwrap().clone())
    }
}
