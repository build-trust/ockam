use crate::ValueStorage;
use ockam_core::compat::{boxed::Box, sync::Arc, sync::RwLock};
use ockam_core::{async_trait, Result};

/// In memory implementation of a value storage
pub struct InMemoryValueStorage<V> {
    storage: Arc<RwLock<V>>,
}

#[async_trait]
impl<V: Send + Sync + Clone + 'static> ValueStorage<V, V> for InMemoryValueStorage<V> {
    async fn update_value(&self, f: impl Fn(V) -> Result<V> + Send + Sync + 'static) -> Result<()> {
        let _ = self
            .modify_value(move |v| {
                let updated = f(v)?;
                Ok((updated.clone(), updated))
            })
            .await?;
        Ok(())
    }

    async fn modify_value(
        &self,
        f: impl Fn(V) -> Result<(V, V)> + Send + Sync + 'static,
    ) -> Result<V> {
        let mut value = self.storage.write().unwrap();
        let (updated, result) = f(value.clone())?;
        *value = updated;
        Ok(result)
    }

    async fn read_value(&self, f: impl Fn(V) -> Result<V> + Send + Sync + 'static) -> Result<V> {
        f(self.storage.read().unwrap().clone())
    }
}

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
    #[allow(non_snake_case)]
    async fn test_vault_synchronization() -> Result<()> {
        let storage = InMemoryValueStorage::<Value>::create();

        let initial = storage.read_value(move |value: Value| Ok(value)).await?;

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
        let updated = storage.read_value(move |value: Value| Ok(value)).await?;
        assert_eq!(updated, Value(10));

        Ok(())
    }

    #[derive(Serialize, Deserialize, Default, PartialEq, Eq, Debug, Clone)]
    struct Value(u8);
}
