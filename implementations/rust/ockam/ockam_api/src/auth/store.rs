#[cfg(feature = "lmdb")]
pub mod lmdb;
pub mod mem;

use ockam_core::async_trait;

#[async_trait]
pub trait Storage {
    type Error: ockam_core::compat::error::Error + Send + Sync + 'static;

    async fn get(&self, id: &str, key: &str) -> Result<Option<Vec<u8>>, Self::Error>;
    async fn set(&self, id: &str, key: String, val: Vec<u8>) -> Result<(), Self::Error>;
    async fn del(&self, id: &str, key: &str) -> Result<(), Self::Error>;
}
