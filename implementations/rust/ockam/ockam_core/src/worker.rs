use crate::lib::Box;
use crate::{Message, Result};

/// Base ockam worker trait.  See [`Handler`] for message receival
///
/// [`Handler`]: self::Handler
#[async_trait::async_trait]
pub trait Worker: Send + 'static {
    type Message: Message;
    type Context: Send + 'static;

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
}
