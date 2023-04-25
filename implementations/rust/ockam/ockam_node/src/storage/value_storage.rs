use ockam_core::{async_trait, compat::boxed::Box, Result};

/// This trait defines a simple storage interface for serializable values
#[async_trait]
pub trait ValueStorage<V, R>: Sync + Send + 'static {
    /// Once the storage has been initialized the contained value can
    /// be updated with this function. The updated value is computed from the
    /// previous value and stored
    async fn update_value(&self, f: impl Fn(V) -> Result<V> + Send + Sync + 'static) -> Result<()>;

    /// Once the storage has been initialized the contained value can
    /// be modified with this function. The updated value is computed from the
    //  previous value and stored. Additionally a result can be returned
    async fn modify_value(
        &self,
        f: impl Fn(V) -> Result<(V, R)> + Send + Sync + 'static,
    ) -> Result<R>;

    /// Read the currently stored value and either return the full value or a subset of it
    async fn read_value(&self, f: impl Fn(V) -> Result<R> + Send + Sync + 'static) -> Result<R>;
}
