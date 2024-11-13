use crate::channel_types::MessageReceiver;
use crate::tokio::runtime::Handle;
use core::sync::atomic::AtomicUsize;
use ockam_core::compat::collections::HashMap;
use ockam_core::compat::sync::{Arc, RwLock};
use ockam_core::compat::time::Duration;
use ockam_core::compat::vec::Vec;
use ockam_core::flow_control::FlowControls;
#[cfg(feature = "std")]
use ockam_core::OpenTelemetryContext;
use ockam_core::{
    async_trait, Address, AddressMetadata, Error, Mailboxes, RelayMessage, Result, TransportType,
};

use crate::router::Router;
#[cfg(feature = "std")]
use core::fmt::{Debug, Formatter};
use ockam_core::compat::sync::Weak;
use ockam_core::errcode::{Kind, Origin};
use ockam_transport_core::Transport;

/// A default timeout in seconds
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// Context mode depending on the fact if it's attached to a Worker or a Processor
#[derive(Clone, Copy, Debug)]
pub enum ContextMode {
    /// Without a Worker or a Processor
    Detached,
    /// With a Worker or a Processor
    Attached,
}

/// Higher value means the worker is shutdown earlier
#[derive(Debug, Default, Clone, Copy, Eq, PartialEq)]
pub enum WorkerShutdownPriority {
    /// 1
    Priority1,
    /// 2
    Priority2,
    /// 3
    Priority3,
    /// 4
    #[default]
    Priority4,
    /// 5
    Priority5,
    /// 6
    Priority6,
    /// 7
    Priority7,
}

impl WorkerShutdownPriority {
    /// All possible values in descending order
    pub fn all_descending_order() -> [WorkerShutdownPriority; 7] {
        use WorkerShutdownPriority::*;
        [
            Priority7, Priority6, Priority5, Priority4, Priority3, Priority2, Priority1,
        ]
    }
}

/// Context contains Node state and references to the runtime.
pub struct Context {
    pub(super) mailboxes: Mailboxes,
    pub(super) router: Weak<Router>,
    pub(super) runtime_handle: Handle,
    pub(super) receiver: MessageReceiver<RelayMessage>,
    pub(super) mailbox_count: Arc<AtomicUsize>,
    /// List of transports used to resolve external addresses to local workers in routes
    pub(super) transports: Arc<RwLock<HashMap<TransportType, Arc<dyn Transport>>>>,
    pub(super) flow_controls: FlowControls,
    pub(super) mode: ContextMode,
    #[cfg(feature = "std")]
    pub(super) tracing_context: OpenTelemetryContext,
}

/// This trait can be used to integrate transports into a node
#[async_trait]
pub trait HasContext {
    /// Return a cloned context
    fn get_context(&self) -> &Context;
}

#[cfg(feature = "std")]
impl Debug for Context {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Context")
            .field("mailboxes", &self.mailboxes)
            .field("runtime", &self.runtime_handle)
            .field("mode", &self.mode)
            .finish()
    }
}

impl Context {
    /// Return runtime clone
    pub fn runtime(&self) -> &Handle {
        &self.runtime_handle
    }

    /// Return mailbox_count clone
    pub(crate) fn mailbox_count(&self) -> Arc<AtomicUsize> {
        self.mailbox_count.clone()
    }

    /// Reference to the Router
    pub(crate) fn router(&self) -> Result<Arc<Router>> {
        self.router
            .upgrade()
            .ok_or_else(|| Error::new(Origin::Node, Kind::Shutdown, "Failed to upgrade router"))
    }

    /// Weak reference to the Router
    pub(crate) fn router_weak(&self) -> Weak<Router> {
        self.router.clone()
    }

    /// Return the primary address of the current worker
    pub fn primary_address(&self) -> &Address {
        self.mailboxes.primary_address()
    }

    /// Return additional addresses of the current worker
    pub fn additional_addresses(&self) -> impl Iterator<Item = &Address> {
        self.mailboxes.additional_addresses()
    }

    /// Return a reference to the mailboxes of this context
    pub fn mailboxes(&self) -> &Mailboxes {
        &self.mailboxes
    }

    /// Shared [`FlowControls`] instance
    pub fn flow_controls(&self) -> &FlowControls {
        &self.flow_controls
    }

    /// Return the tracing context
    #[cfg(feature = "std")]
    pub fn tracing_context(&self) -> OpenTelemetryContext {
        self.tracing_context.clone()
    }

    /// Set the current tracing context
    #[cfg(feature = "std")]
    pub fn set_tracing_context(&mut self, tracing_context: OpenTelemetryContext) {
        self.tracing_context = tracing_context
    }
}

impl Context {
    /// Return a list of all available worker addresses on a node
    pub fn list_workers(&self) -> Result<Vec<Address>> {
        Ok(self.router()?.list_workers())
    }

    /// Return true if a worker is already registered at this address
    pub fn is_worker_registered_at(&self, address: &Address) -> Result<bool> {
        Ok(self.router()?.is_worker_registered_at(address))
    }

    /// Finds the terminal address of a route, if present
    pub fn find_terminal_address<'a>(
        &self,
        addresses: impl Iterator<Item = &'a Address>,
    ) -> Result<Option<(&'a Address, AddressMetadata)>> {
        Ok(self.router()?.find_terminal_address(addresses))
    }

    /// Read metadata for the provided address
    pub fn get_metadata(&self, address: &Address) -> Result<Option<AddressMetadata>> {
        Ok(self.router()?.get_address_metadata(address))
    }
}
