use ockam_core::compat::boxed::Box;
use ockam_core::{Address, Message, Result, Routed};

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
    /// Run an initialise hook
    ///
    /// This gives this SystemHandler the chance to setup any state on
    /// the worker or send messages to other workers (and hooks).
    /// Finally it must return the address it wants to respond to
    async fn initialize(&mut self, ctx: &mut C) -> Result<Address>;

    /// Called for every message addressed to the system handler
    async fn handle_message(&mut self, ctx: &mut C, msg: Routed<M>) -> Result<()>;
}
