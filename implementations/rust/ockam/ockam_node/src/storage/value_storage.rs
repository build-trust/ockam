use ockam_core::{async_trait, compat::boxed::Box, Result};

/// This trait defines a simple storage interface for serializable values
#[async_trait]
pub trait ValueStorage<V>: Sync + Send + 'static {
    /// Once the storage has been initialized the contained value can
    /// be updated with this function. The update function must
    /// return
    ///   - the updated value to store
    ///   - an additional result which can be for example computed from the previously stored value
    async fn update_value<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(V) -> Result<(V, R)> + Send + 'static,
        R: Send + 'static;

    /// Read the currently stored value and either return the full value or a subset of it
    async fn read_value<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(V) -> Result<R> + Send + 'static,
        R: Send + 'static;
}
