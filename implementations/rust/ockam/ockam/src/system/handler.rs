use ockam_core::compat::boxed::Box;
use ockam_core::{Message, Result, Routed};

/// Handle a single type of message for a worker system-address
///
/// A handle may re-emit messages to the worker system, or to the
/// Ockam runtime.  All state associated with a particular protocol
/// must be contained in the type that implements this trait.
#[ockam_core::async_trait]
pub trait SystemHandler<C, M>
where
    C: Send + 'static,
    M: Message,
{
    /// Called for every message addressed to the system handler
    async fn handle_message(&mut self, ctx: &mut C, msg: Routed<M>) -> Result<()>;
}
