use ockam_core::{async_trait, Result};

#[async_trait::async_trait]
pub trait Transport {
    async fn connect(&self, peer: &str) -> Result<()>;
    async fn listen(&self, addr: &str) -> Result<()>;
    async fn shutdown(&self) -> Result<()>;
}
