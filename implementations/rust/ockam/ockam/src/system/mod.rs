//! Worker system module

mod handler;
pub use handler::SystemHandler;

mod builder;
pub use builder::SystemBuilder;

#[cfg(test)]
mod tests;

use crate::OckamError;
use ockam_core::compat::{boxed::Box, collections::BTreeMap};
use ockam_core::{Address, Result, Routed, Worker};

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
