use crate::compat::boxed::Box;
use crate::{async_trait, Result};

/// Defines an interface for Ockam Workers that need to continuously
/// perform background operations.
#[async_trait]
pub trait Processor: Send + 'static {
    /// The Ockam API Context and other resources available for the
    /// processor during processing.
    type Context: Send + 'static;

    /// Define the Processor Worker initialisation behaviour.
    async fn initialize(&mut self, _context: &mut Self::Context) -> Result<()> {
        Ok(())
    }

    /// Define the Processor Worker shutdown behaviour.
    async fn shutdown(&mut self, _context: &mut Self::Context) -> Result<()> {
        Ok(())
    }

    /// Define the Processor Worker background execution behaviour.
    ///
    /// The `process()` callback function allows you to define worker
    /// behavior that will be executed.
    ///
    /// If no `.await` is performed during `process()`, the execution
    /// will result in a busy loop.
    ///
    /// It's important to not block this function for long periods of
    /// time as it is co-operatively scheduled by the underlying async
    /// runtime and will block all other Ockam Node operations until
    /// it returns.
    ///
    /// Always prefer async `.await` operations, as blocking operations
    /// can cause deadlocks.
    async fn process(&mut self, _context: &mut Self::Context) -> Result<bool> {
        Ok(false)
    }
}
