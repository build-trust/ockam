mod processor;
mod record;
mod shutdown;
mod state;
pub mod worker;

#[cfg(feature = "metrics")]
use core::sync::atomic::AtomicUsize;

use crate::channel_types::{MessageSender, OneshotSender};
use crate::relay::CtrlSignal;
use crate::tokio::runtime::Handle;
use crate::{NodeError, NodeReason};
use alloc::string::String;
use alloc::vec::Vec;
use ockam_core::compat::collections::hash_map::Entry;
use ockam_core::compat::collections::HashMap;
use ockam_core::compat::sync::RwLock as SyncRwLock;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::flow_control::FlowControls;
use ockam_core::{Address, AddressMetadata, Error, RelayMessage, Result, TransportType};
use record::{AddressRecord, InternalMap, WorkerMeta};
use state::{NodeState, RouterState};

/// A pair of senders to a worker relay
#[derive(Debug)]
pub struct SenderPair {
    pub msgs: MessageSender<RelayMessage>,
    pub ctrl: OneshotSender<CtrlSignal>,
}

/// A combined address type and local worker router
///
/// This router supports two routing modes: internal, and external.
///
/// Internal routing resolves `type=0` addresses to local workers.
///
/// External routing is supported only after a plugin component
/// registers itself with this router.  Only one router can be
/// registered per address type.
pub struct Router {
    runtime_handle: Handle,
    /// Keep track of some additional router state information
    state: RouterState,
    /// Internal address state
    map: InternalMap,
    /// Externally registered router components
    external: SyncRwLock<HashMap<TransportType, Address>>,
}

enum RouteType {
    Internal,
    External(TransportType),
}

fn determine_type(next: &Address) -> RouteType {
    if next.transport_type().is_local() {
        RouteType::Internal
    } else {
        RouteType::External(next.transport_type())
    }
}

impl Router {
    pub fn new(runtime_handle: Handle, flow_controls: &FlowControls) -> Self {
        Self {
            runtime_handle,
            state: RouterState::new(),
            map: InternalMap::new(flow_controls),
            external: Default::default(),
        }
    }

    pub fn set_cluster(&self, addr: &Address, label: String) -> Result<()> {
        self.map.set_cluster(addr, label)
    }

    pub fn list_workers(&self) -> Vec<Address> {
        self.map.list_workers()
    }

    pub fn is_worker_registered_at(&self, address: &Address) -> bool {
        self.map.is_worker_registered_at(address)
    }

    pub fn stop_ack(&self, primary_address: &Address) -> Result<()> {
        let running = self.state.is_running();
        debug!(%running, "Handling shutdown ACK for {}", primary_address);

        let empty = self.map.stop_ack(primary_address);
        if !running && empty {
            self.state.set_to_stopped();
        }

        Ok(())
    }

    pub fn find_terminal_address<'a>(
        &self,
        addresses: impl Iterator<Item = &'a Address>,
    ) -> Option<(&'a Address, AddressMetadata)> {
        self.map.find_terminal_address(addresses)
    }

    pub fn get_address_metadata(&self, address: &Address) -> Option<AddressMetadata> {
        self.map.get_address_metadata(address)
    }

    pub fn register_router(&self, tt: TransportType, addr: Address) -> Result<()> {
        if let Entry::Vacant(e) = self.external.write().unwrap().entry(tt) {
            e.insert(addr);
            Ok(())
        } else {
            // already exists
            Err(Error::new(
                Origin::Node,
                Kind::AlreadyExists,
                "Router already exists",
            ))
        }
    }

    pub fn resolve(&self, addr: &Address) -> Result<MessageSender<RelayMessage>> {
        let addr = match determine_type(addr) {
            RouteType::Internal => addr,
            // TODO: Remove after other transport implementations are moved to new architecture
            RouteType::External(tt) => &self.address_for_transport(tt)?,
        };
        self.map.resolve(addr)
    }

    fn address_for_transport(&self, tt: TransportType) -> Result<Address> {
        let guard = self.external.read().unwrap();
        guard
            .get(&tt)
            .cloned()
            .ok_or_else(|| NodeError::NodeState(NodeReason::Unknown).internal())
    }

    pub async fn wait_termination(&self) {
        self.state.wait_until_stopped().await;
    }

    /// Stop the worker
    pub fn stop_address(&self, addr: &Address, skip_sending_stop_signal: bool) -> Result<()> {
        debug!("Stopping address '{}'", addr);

        self.map.stop(addr, skip_sending_stop_signal)?;

        Ok(())
    }

    #[cfg(feature = "metrics")]
    pub(crate) fn get_metrics_readout(&self) -> (Arc<AtomicUsize>, Arc<AtomicUsize>) {
        self.map.get_metrics()
    }
}
