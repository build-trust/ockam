use ockam_core::async_trait;
use ockam_core::{AsyncTryClone, Result};

/// Storage for Authenticated data
#[async_trait]
pub trait AuthenticatedStorage: AsyncTryClone + Send + Sync + 'static {
    /// Get entry
    async fn get(&self, id: &str, key: &str) -> Result<Option<Vec<u8>>>;

    /// Set entry
    async fn set(&self, id: &str, key: String, val: Vec<u8>) -> Result<()>;

    /// Delete entry
    async fn del(&self, id: &str, key: &str) -> Result<()>;
}

/// In-memory impl
pub mod mem;
