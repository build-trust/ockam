use crate::ValueStorage;
use ockam_core::compat::{boxed::Box, sync::Arc, sync::RwLock};
use ockam_core::{async_trait, Result};

/// In memory implementation of a value storage
pub struct InMemoryValueStorage<V> {
    storage: Arc<RwLock<V>>,
}

/// Trait implementation for ValueStorage
#[async_trait]
impl<V: Send + Sync + Clone + 'static> ValueStorage<V> for InMemoryValueStorage<V> {
    async fn update_value(&self, f: impl Fn(V) -> Result<V> + Send + Sync + 'static) -> Result<()> {
        let _ = self
            .modify_value(move |v| {
                let updated = f(v)?;
                Ok((updated.clone(), updated))
            })
            .await?;
        Ok(())
    }

    async fn modify_value<R>(
        &self,
        f: impl Fn(V) -> Result<(V, R)> + Send + Sync + 'static,
    ) -> Result<R> {
        let mut value = self.storage.write().unwrap();
        let (updated, result) = f(value.clone())?;
        *value = updated;
        Ok(result)
    }

    async fn read_value<R>(&self, f: impl Fn(V) -> Result<R> + Send + Sync + 'static) -> Result<R> {
        f(self.storage.read().unwrap().clone())
    }
}

/// Default in-memory value storage, starting with a default value for V
impl<V: Default> InMemoryValueStorage<V> {
    /// Create a new in-memory value storage
    pub fn create() -> InMemoryValueStorage<V> {
        InMemoryValueStorage {
            storage: Default::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ockam_core::Result;
    use serde::{Deserialize, Serialize};

    #[tokio::test]
    async fn test_value_storage() -> Result<()> {
        let storage = InMemoryValueStorage::<Value>::create();

        let initial = storage.read_value(Ok).await?;

        // sanity check
        assert_eq!(Value::default(), Value(0));

        // the initial value is the default value
        assert_eq!(initial, Value::default());

        // the value can be updated
        storage
            .update_value(move |_: Value| Ok(Value(10)))
            .await
            .unwrap();

        // the new value can be read again
        let updated = storage.read_value(Ok).await?;
        assert_eq!(updated, Value(10));

        Ok(())
    }

    #[derive(Serialize, Deserialize, Default, PartialEq, Eq, Debug, Clone)]
    struct Value(u8);
}
