use crate::Context;
use ockam_core::{async_trait, compat::boxed::Box, Message, Result, Routed};

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

    /// Override initialisation behaviour.
    async fn initialize(&mut self, _context: &mut Context) -> Result<()> {
        Ok(())
    }

    /// Override shutdown behaviour.
    async fn shutdown(&mut self, _context: &mut Context) -> Result<()> {
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
    async fn process(&mut self, ctx: &mut Context) -> Result<bool> {
        match recv_message(ctx).await? {
            Some(message) => {
                self.handle_message(ctx, message).await?;
                Ok(true)
            }
            None => Ok(false),
        }
    }

    /// Try to open and handle a typed message.
    async fn handle_message(
        &mut self,
        _context: &mut Context,
        _msg: Routed<Self::Message>,
    ) -> Result<()> {
        Ok(())
    }
}

/// Receive and handle a single message
///
/// Report errors as they occur, and signal whether the loop should
/// continue running or not
async fn recv_message<M: Message>(ctx: &mut Context) -> Result<Option<Routed<M>>> {
    let relay_msg = match ctx.receiver_next().await? {
        Some(msg) => msg,
        None => {
            trace!("No more messages for worker {}", ctx.primary_address());
            return Ok(None);
        }
    };

    // Call the worker handle function - pass errors up
    #[cfg(feature = "std")]
    {
        let tracing_context = relay_msg.local_message().tracing_context();
        // We set the tracing context retrieved from the local message on the worker context
        // This way, if the worker invokes ctx.send_message() to send a message to another worker,
        // that same tracing context will be passed along when a LocalMessage will be created
        // (see send_from_address_impl)
        ctx.set_tracing_context(tracing_context.clone());
    }

    Ok(Some(Routed::new(
        relay_msg.destination().clone(),
        relay_msg.source().clone(),
        relay_msg.into_local_message(),
    )))
}
