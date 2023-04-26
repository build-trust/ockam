use ockam_core::{async_trait, compat::boxed::Box, Result};

/// This trait defines a storage interface for serializable values
/// It uses a closures in its interface in order to support a transactional behaviour and recovery from
/// errors. If the closure does not return a successful value then no change is performed
///
/// A ValueStorage is always supposed to contain a value of type V, which can then be read or modified
#[async_trait]
pub trait ValueStorage<V>: Sync + Send + 'static {
    /// Update the currently stored value
    async fn update_value(&self, f: impl Fn(V) -> Result<V> + Send + Sync + 'static) -> Result<()>;

    /// Update the currently stored value and return a result R
    async fn modify_value<R: Send + Sync + 'static>(
        &self,
        f: impl Fn(V) -> Result<(V, R)> + Send + Sync + 'static,
    ) -> Result<R>;

    /// Read the currently stored value and either return the full value or a subset of it, as R
    async fn read_value<R: Send + Sync + 'static>(
        &self,
        f: impl Fn(V) -> Result<R> + Send + Sync + 'static,
    ) -> Result<R>;
}
