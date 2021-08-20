use crate::Result;
use async_trait::async_trait;

/// Base ockam processor trait.
#[async_trait]
pub trait Processor: Send + 'static {
    /// The API and other resources available for the processor during processing.
    type Context: Send + 'static;

    /// Override initialisation behaviour
    async fn initialize(&mut self, _context: &mut Self::Context) -> Result<()> {
        Ok(())
    }

    /// Override shutdown behaviour
    async fn shutdown(&mut self, _context: &mut Self::Context) -> Result<()> {
        Ok(())
    }

    /// Process
    async fn process(&mut self, _context: &mut Self::Context) -> Result<bool> {
        Ok(false)
    }
}
