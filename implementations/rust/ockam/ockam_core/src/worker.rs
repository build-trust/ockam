use crate::{lib::Box, Message, Result};
use async_trait::async_trait;

/// Base ockam worker trait.
#[async_trait]
pub trait Worker: Send + 'static {
    type Message: Message;
    type Context: Context + Send + 'static;

    /// Override initialisation behaviour
    fn initialize(&mut self, _context: &mut Self::Context) -> Result<()> {
        Ok(())
    }

    /// Override shutdown behaviour
    fn shutdown(&mut self, _context: &mut Self::Context) -> Result<()> {
        Ok(())
    }

    /// Try to open and handle a typed message
    async fn handle_message(
        &mut self,
        _context: &mut Self::Context,
        _msg: Self::Message,
    ) -> Result<()> {
        Ok(())
    }

    /// This function is called for errors reported to this worker as a supervisor
    fn handle_failures(&mut self, _context: &mut Self::Context, _msg: Error) {}
}
