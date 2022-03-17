//! Worker system module

#[cfg(test)]
mod tests;

use crate::OckamError;
use ockam_core::compat::{boxed::Box, collections::BTreeMap};
use ockam_core::{Address, Message, Result, Routed, Worker};

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

/// A composable worker system type
///
/// A worker system is a single worker which can act as a full cluster
/// of workers.  This is achieved via the `send_to_self(...)` API
/// endpoint on the Ockam Node API.
///
/// The worker system also provides some utilities for creating and
/// managing mappings between hidden API-addresses and behaviour hooks
/// associated to each address.
///
/// The advantage of a worker system over a full set of workers is a
/// lower memory overhead for resource constrained devices.
pub struct WorkerSystem<W: Worker> {
    map: BTreeMap<Address, Box<dyn SystemHandler<W::Context, W::Message> + Send + 'static>>,
}

impl<W: Worker> Default for WorkerSystem<W> {
    fn default() -> Self {
        Self {
            map: BTreeMap::new(),
        }
    }
}

impl<W: Worker> WorkerSystem<W> {
    /// Attach a system handler to this system
    pub fn attach<A, H>(&mut self, addr: A, handler: H)
    where
        A: Into<Address>,
        H: SystemHandler<W::Context, W::Message> + Send + 'static,
    {
        self.map.insert(addr.into(), Box::new(handler));
    }

    /// Handle a message via this worker system
    pub async fn handle_message(
        &mut self,
        ctx: &mut W::Context,
        msg: Routed<W::Message>,
    ) -> Result<()> {
        let addr = msg.msg_addr();
        match self.map.get_mut(&addr) {
            Some(handle) => handle.handle_message(ctx, msg).await,
            None => Err(OckamError::SystemAddressNotBound.into()),
        }
    }
}
