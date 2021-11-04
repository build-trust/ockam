use crate::compat::boxed::Box;
use crate::{async_trait, NodeContext, Result};

/// Base ockam processor trait.
#[async_trait]
pub trait Processor<C: NodeContext>: Send + 'static {
    /// Override initialisation behaviour
    async fn initialize(&mut self, _context: &mut C) -> Result<()> {
        Ok(())
    }

    /// Override shutdown behaviour
    async fn shutdown(&mut self, _context: &mut C) -> Result<()> {
        Ok(())
    }

    /// Process
    async fn process(&mut self, _context: &mut C) -> Result<bool> {
        Ok(false)
    }
}
