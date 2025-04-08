use crate::channel_types::MessageReceiver;
use crate::context::ContextState;
use crate::router::Router;
use crate::tokio::runtime::Handle;
use crate::ContextMode;
#[cfg(feature = "std")]
use core::fmt::{Debug, Formatter};
use core::sync::atomic::AtomicUsize;
use ockam_core::compat::collections::HashMap;
use ockam_core::compat::sync::Weak;
use ockam_core::compat::sync::{Arc, RwLock};
use ockam_core::compat::time::Duration;
use ockam_core::compat::vec::Vec;
use ockam_core::flow_control::FlowControls;
#[cfg(feature = "std")]
use ockam_core::OpenTelemetryContext;
use ockam_core::{Address, AddressMetadata, Mailboxes, RelayMessage, Result, TransportType};
use ockam_transport_core::Transport;
#[cfg(feature = "std")]
use opentelemetry::trace::Span;

/// A default timeout in seconds
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// Context contains Node state and references to the runtime.
pub struct Context {
    pub(super) receiver: MessageReceiver<RelayMessage>,
    pub(super) state: Arc<ContextState>,
    /// List of transports used to resolve external addresses to local workers in routes
    pub(super) transports: Arc<RwLock<HashMap<TransportType, Arc<dyn Transport>>>>,
    pub(super) runtime_handle: Handle,
    pub(super) mode: ContextMode,
}

#[cfg(feature = "std")]
impl Debug for Context {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Context")
            .field("mailboxes", &self.state.mailboxes)
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
        self.state.mailbox_count.clone()
    }

    /// Reference to the Router
    pub(crate) fn router(&self) -> Result<Arc<Router>> {
        self.state.router()
    }

    /// Weak reference to the Router
    pub(crate) fn router_weak(&self) -> Weak<Router> {
        self.state.router.clone()
    }

    /// Return the primary address of the current worker
    pub fn primary_address(&self) -> &Address {
        self.state.primary_address()
    }

    /// Return additional addresses of the current worker
    pub fn additional_addresses(&self) -> impl Iterator<Item = &Address> {
        self.state.mailboxes.additional_addresses()
    }

    /// Return a reference to the mailboxes of this context
    pub fn mailboxes(&self) -> &Mailboxes {
        self.state.mailboxes()
    }

    /// Shared [`FlowControls`] instance
    pub fn flow_controls(&self) -> &FlowControls {
        &self.state.flow_controls
    }

    /// Return the tracing context
    #[cfg(feature = "std")]
    pub fn tracing_context(&self) -> OpenTelemetryContext {
        self.state.tracing_context()
    }

    /// Set the current tracing context
    #[cfg(feature = "std")]
    pub fn set_tracing_context(&self, tracing_context: OpenTelemetryContext) {
        self.state.set_tracing_context(tracing_context);
    }

    /// Set the current tracing context from a started span
    #[cfg(feature = "std")]
    pub fn set_tracing_context_from_span(&self, span: impl Span) {
        self.state.set_tracing_context_from_span(span);
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
