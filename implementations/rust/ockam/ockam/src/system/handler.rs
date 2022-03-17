use dyn_clonable::*;
use dyn_clone::DynClone;
use ockam_core::compat::{boxed::Box, collections::BTreeMap, string::String};
use ockam_core::{Address, Message, Result, Routed};

/// Handle a single type of message for a worker system-address
///
/// A handle may re-emit messages to the worker system, or to the
/// Ockam runtime.  All state associated with a particular protocol
/// must be contained in the type that implements this trait.
///
/// A SystemHandler is able to send messages to both external workers,
/// and other internal handlers.  To allow workers to create behaviour
/// pipelines, we need to pre-define "routes" for a SystemHandler
/// (i.e. where is a message sent after it is done processing it).
///
/// In most cases this only requires a "default" route, but may use
/// different routing labels in more complicated setups (to build
/// processing graphs, instead of pipelines).
///
/// It is highly recommended to use the
/// [SystemBuilder](crate::SystemBuilder) utility to generate this
/// information.
#[clonable]
#[ockam_core::async_trait]
pub trait SystemHandler<C, M>: Clone + DynClone
where
    C: Send + 'static,
    M: Message,
{
    /// Setup internal route path for this handler
    ///
    /// This function is only called once with a route map.  To
    /// generate this route map see the
    /// [SystemBuilder](crate::SystemBuilder) utility.
    async fn initialize(
        &mut self,
        ctx: &mut C,
        routes: &mut BTreeMap<String, Address>,
    ) -> Result<()>;

    /// Called for every message addressed to the system handler
    async fn handle_message(
        &mut self,
        self_addr: Address,
        ctx: &mut C,
        msg: Routed<M>,
    ) -> Result<()>;
}
