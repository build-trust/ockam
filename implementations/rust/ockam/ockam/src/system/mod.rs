//! Worker system module

mod handler;
pub use handler::SystemHandler;

mod builder;
pub use builder::SystemBuilder;

#[cfg(test)]
mod tests;

use crate::OckamError;
use ockam_core::compat::{boxed::Box, collections::BTreeMap};
use ockam_core::{Address, Message, Result, Routed};

/// A componasble worker system type
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
pub struct WorkerSystem<C: Send + 'static, M: Message> {
    map: BTreeMap<Address, Box<dyn SystemHandler<C, M> + Send + 'static>>,
}

impl<C: Send + 'static, M: Message> Default for WorkerSystem<C, M> {
    fn default() -> Self {
        Self {
            map: BTreeMap::new(),
        }
    }
}

impl<C: Send + 'static, M: Message> WorkerSystem<C, M> {
    /// Attach a system handler to this system
    pub fn attach<A, H>(&mut self, addr: A, handler: H)
    where
        A: Into<Address>,
        H: SystemHandler<C, M> + Send + 'static,
    {
        self.map.insert(addr.into(), Box::new(handler));
    }

    /// Attach a boxed system handler to this system
    pub fn attach_boxed<A: Into<Address>>(
        &mut self,
        addr: A,
        handler: Box<dyn SystemHandler<C, M> + Send + 'static>,
    ) {
        self.map.insert(addr.into(), handler);
    }

    /// Handle a message via this worker system
    pub async fn handle_message(&mut self, ctx: &mut C, msg: Routed<M>) -> Result<()> {
        let addr = msg.msg_addr();
        match self.map.get_mut(&addr) {
            Some(handle) => handle.handle_message(ctx, msg).await,
            None => Err(OckamError::SystemAddressNotBound.into()),
        }
    }
}
