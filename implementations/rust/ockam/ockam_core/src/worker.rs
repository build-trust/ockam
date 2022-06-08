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
    async fn initialize(&mut self, _context: &mut Self::Context) -> Result<()> {
        Ok(())
    }

    /// Override shutdown behaviour.
    async fn shutdown(&mut self, _context: &mut Self::Context) -> Result<()> {
        Ok(())
    }

    /// Try to authorize an incoming message
    ///
    /// The authorization flow of an incoming message looks like this:
    ///
    /// 1. [`WorkerRelay::recv_message`] requests the next message from
    ///    its associated `Context`.
    /// 1. [`Context::receiver_next`] pulls the incoming message from
    ///    its [`SmallReceiver`] channel.
    /// 1. [`Context::receiver_next`] then verifies the `AccessControl` rules
    ///    associated with its [`Mailboxes`] are valid before returning the
    ///    message to `WorkerRelay`.
    /// 1. [`WorkerRelay::recv_message`] invokes this `is_authorized` function
    ///    and only invokes `Worker::handle_message` if `ockam_core::allowed()`
    ///    is returned.
    /// 1. If the message is not authorized it will be silently dropped and a
    ///    warning message output to the worker log.
    #[allow(clippy::wrong_self_convention)]
    async fn is_authorized(
        &mut self,
        _context: &mut Self::Context,
        _msg: Routed<Self::Message>,
    ) -> Result<bool> {
        crate::allow()
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
