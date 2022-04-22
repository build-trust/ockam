//! Types and traits for workers which act as a cluster of workers,
//! [`WorkerSystem`]s.

mod handler;
pub use handler::SystemHandler;

mod builder;
pub use builder::SystemBuilder;

pub mod hooks;

#[cfg(test)]
mod tests;

use crate::OckamError;
use ockam_core::compat::{boxed::Box, collections::BTreeMap, vec::Vec};
use ockam_core::{Address, Message, Result, Routed};

/// A composable worker system type.
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
    entry: Option<Address>,
}

impl<C: Send + 'static, M: Message> Clone for WorkerSystem<C, M> {
    fn clone(&self) -> Self {
        Self {
            entry: self.entry.clone(),
            map: self
                .map
                .iter()
                .map(|(addr, h)| (addr.clone(), *dyn_clone::clone_box(&*h)))
                .collect(),
        }
    }
}

impl<C: Send + 'static, M: Message> Default for WorkerSystem<C, M> {
    fn default() -> Self {
        Self {
            map: BTreeMap::new(),
            entry: None,
        }
    }
}

impl<C: Send + 'static, M: Message> WorkerSystem<C, M> {
    /// Check whether this system has registered handlers
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Return the set of used addresses in this system
    pub fn addresses(&self) -> Vec<Address> {
        self.map.keys().map(Clone::clone).collect()
    }

    /// Attach a system handler to this system
    pub fn attach<H>(&mut self, addr: Address, handler: H)
    where
        H: SystemHandler<C, M> + Send + 'static,
    {
        self.map.insert(addr, Box::new(handler));
    }

    /// Attach a boxed system handler to this system
    pub fn attach_boxed(
        &mut self,
        addr: Address,
        handler: Box<dyn SystemHandler<C, M> + Send + 'static>,
    ) {
        self.map.insert(addr, handler);
    }

    /// Specify an "entry point" address for this system
    ///
    /// Because a worker system is a graph of relationships between
    /// ['SystemHandler'](crate::SystemHandler) instances it may be
    /// hard to determine which instance to send a message to first.
    /// The pre-configuration phase of a worker system can determine
    /// this address and store it as the entry-point.
    ///
    /// You can then start the handling process by calling
    /// `dispatch_entry()`.
    pub fn set_entry(&mut self, addr: Address) {
        self.entry = Some(addr);
    }

    /// Get an optional reference to the entry point of this system
    pub fn entrypoint(&self) -> Option<&Address> {
        self.entry.as_ref()
    }

    /// Dispatch a message to the pre-configured system entry point
    ///
    /// This function returns an error if no entry point was
    /// configured or the configured address was not bound.
    pub async fn dispatch_entry(&mut self, ctx: &mut C, msg: Routed<M>) -> Result<()> {
        match self
            .entry
            .as_ref()
            .and_then(|entry| self.map.get_mut(entry).map(|h| (entry, h)))
        {
            Some((addr, handle)) => handle.handle_message(addr.clone(), ctx, msg).await,
            None => Err(OckamError::SystemAddressNotBound.into()),
        }
    }

    /// Handle a message via this worker system
    pub async fn handle_message(&mut self, ctx: &mut C, msg: Routed<M>) -> Result<()> {
        let addr = msg.msg_addr();
        match self.map.get_mut(&addr) {
            Some(handle) => handle.handle_message(addr, ctx, msg).await,
            None => Err(OckamError::SystemAddressNotBound.into()),
        }
    }
}
