//! Worker system module

mod handler;
pub use handler::SystemHandler;

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

impl<W: Worker> WorkerSystem<W> {
    /// Setup this worker system state
    pub async fn setup<I>(&mut self, ctx: &mut W::Context, iter: I) -> Result<()>
    where
        I: IntoIterator + Send + 'static,
        <I as IntoIterator>::IntoIter: Send,
        I::Item: SystemHandler<W::Context, W::Message> + Send + 'static,
    {
        for mut handler in iter.into_iter() {
            let addr = handler.initialize(ctx).await?;
            self.map.insert(addr, Box::new(handler));
        }

        Ok(())
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
