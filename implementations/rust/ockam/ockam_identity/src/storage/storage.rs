use ockam_core::async_trait;
use ockam_core::compat::{boxed::Box, string::String, vec::Vec};
use ockam_core::Result;

/// Storage for Authenticated data
#[async_trait]
pub trait Storage: Send + Sync + 'static {
    /// Get entry
    async fn get(&self, id: &str, key: &str) -> Result<Option<Vec<u8>>>;

    /// Set entry
    async fn set(&self, id: &str, key: String, val: Vec<u8>) -> Result<()>;

    /// Delete entry
    async fn del(&self, id: &str, key: &str) -> Result<()>;

    /// List all keys of a given "type".  TODO: we shouldn't store different things on a single
    /// store.
    async fn keys(&self, namespace: &str) -> Result<Vec<String>>;
}
