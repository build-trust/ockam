use crate::{async_trait, compat::boxed::Box, Message, NodeContext, Result, Routed};

/// Base ockam worker trait.
#[async_trait]
pub trait Worker<C: NodeContext>: Send + 'static {
    /// The type of Message the Worker is sent in [`Self::handle_message`]
    type Message: Message;

    /// Override initialisation behaviour
    async fn initialize(&mut self, _context: &mut C) -> Result<()> {
        Ok(())
    }

    /// Override shutdown behaviour
    async fn shutdown(&mut self, _context: &mut C) -> Result<()> {
        Ok(())
    }

    /// Try to open and handle a typed message
    async fn handle_message(
        &mut self,
        _context: &mut C,
        _msg: Routed<Self::Message>,
    ) -> Result<()> {
        Ok(())
    }
}
