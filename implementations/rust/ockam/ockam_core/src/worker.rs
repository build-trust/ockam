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

    /// The API and other resources available for the worker during message
    /// processing.
    ///
    /// Currently, this should be always `ockam::Context` or
    /// `ockam_node::Context` (which are the same type), but in the future
    /// custom node implementations may use a different context type.
    type Context: Send + 'static;

    /// Override initialisation behaviour.
    #[tracing::instrument(skip_all, fields(worker_type = core::any::type_name::<Self>()))]
    async fn initialize(&mut self, _context: &mut Self::Context) -> Result<()> {
        Ok(())
    }

    /// Override shutdown behaviour.
    #[tracing::instrument(skip_all, fields(worker_type = core::any::type_name::<Self>()))]
    async fn shutdown(&mut self, _context: &mut Self::Context) -> Result<()> {
        Ok(())
    }

    /// Try to open and handle a typed message.
    #[tracing::instrument(skip_all, fields(worker_type = core::any::type_name::<Self>()))]
    async fn handle_message(
        &mut self,
        _context: &mut Self::Context,
        _msg: Routed<Self::Message>,
    ) -> Result<()> {
        Ok(())
    }
}
