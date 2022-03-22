use crate::{async_trait, compat::boxed::Box, Message, Result, Routed};

/// Defines the core interface shared by all Ockam Workers.
///
/// While all methods do not need to be implemented, at the very
/// least, the `Context` and `Message` types need to be specified
/// before a worker can be used in any call to a `Context` API such as
/// `context.start_worker(...)`.
#[async_trait]
pub trait Worker: Send + 'static {
    /// The type of Message the Worker is sent in [`Self::handle_message`].
    type Message: Message;

    /// The API and other resources available for the worker during message processing.
    type Context: Send + 'static;

    /// Override initialisation behaviour.
    async fn initialize(&mut self, _context: &mut Self::Context) -> Result<()> {
        Ok(())
    }

    /// Override shutdown behaviour.
    async fn shutdown(&mut self, _context: &mut Self::Context) -> Result<()> {
        Ok(())
    }

    /// Try to open and handle a typed message.
    async fn handle_message(
        &mut self,
        _context: &mut Self::Context,
        _msg: Routed<Self::Message>,
    ) -> Result<()> {
        Ok(())
    }
}
