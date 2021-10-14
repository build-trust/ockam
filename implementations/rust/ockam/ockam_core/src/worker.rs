use crate::{async_trait, compat::boxed::Box, Message, Result, Routed};

/// Base ockam worker trait.
#[async_trait]
pub trait Worker: Send + 'static {
    /// The type of Message the Worker is sent in [`Self::handle_message`]
    type Message: Message;

    /// The API and other resources available for the worker during message processing.
    type Context: Send + 'static;

    /// Override initialisation behaviour
    async fn initialize(&mut self, _context: &mut Self::Context) -> Result<()> {
        Ok(())
    }

    /// Override shutdown behaviour
    async fn shutdown(&mut self, _context: &mut Self::Context) -> Result<()> {
        Ok(())
    }

    /// Try to open and handle a typed message
    async fn handle_message(
        &mut self,
        _context: &mut Self::Context,
        _msg: Routed<Self::Message>,
    ) -> Result<()> {
        Ok(())
    }
}
