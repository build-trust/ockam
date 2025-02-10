#[cfg(feature = "metrics")]
use core::sync::atomic::AtomicUsize;

use super::record::InternalMap;
use crate::channel_types::{MessageSender, OneshotSender};
use crate::relay::CtrlSignal;
use crate::{NodeError, NodeReason};
use alloc::vec::Vec;
use ockam_core::compat::collections::hash_map::Entry;
use ockam_core::compat::collections::HashMap;
use ockam_core::compat::sync::RwLock as SyncRwLock;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::flow_control::FlowControls;
use ockam_core::{Address, AddressMetadata, Error, RelayMessage, Result, TransportType};

/// A pair of senders to a worker relay
#[derive(Debug)]
pub struct SenderPair {
    pub msgs: MessageSender<RelayMessage>,
    pub ctrl: OneshotSender<CtrlSignal>,
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
    /// Keep track of some additional router state information
    pub(super) state: SyncRwLock<RouterState>,
    /// Internal address state
    pub(super) map: InternalMap,
    /// Externally registered router components
    pub(super) external: SyncRwLock<HashMap<TransportType, Address>>,
    #[cfg(feature = "std")]
    pub(super) shutdown_broadcast_sender: SyncRwLock<Option<tokio::sync::broadcast::Sender<()>>>,
}

/// Node state
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RouterState {
    Running,
    ShuttingDown,
    Shutdown,
}

impl Router {
    pub fn new(flow_controls: &FlowControls) -> Self {
        #[cfg(feature = "std")]
        let (shutdown_broadcast_sender, _) = tokio::sync::broadcast::channel(1);

        Self {
            state: RouterState::Running.into(),
            map: InternalMap::new(flow_controls),
            external: Default::default(),
            #[cfg(feature = "std")]
            shutdown_broadcast_sender: SyncRwLock::new(Some(shutdown_broadcast_sender)),
        }
    }

    pub fn list_workers(&self) -> Vec<Address> {
        self.map.list_workers()
    }

    pub fn is_worker_registered_at(&self, address: &Address) -> bool {
        self.map.is_worker_registered_at(address)
    }

    pub fn stop_ack(&self, primary_address: &Address) -> Result<()> {
        debug!("Handling shutdown ACK for {}", primary_address);

        self.map.stop_ack(primary_address);

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

    /// Stop the worker
    pub fn stop_address(&self, addr: &Address, skip_sending_stop_signal: bool) -> Result<()> {
        self.map.stop(addr, skip_sending_stop_signal)
    }

    #[cfg(feature = "std")]
    pub async fn wait_termination(&self) {
        let mut receiver = match self.shutdown_broadcast_sender.read().unwrap().as_ref() {
            None => {
                // That's fine, means we already stopped
                debug!("Waiting for termination but channel is missing");
                return;
            }
            Some(sender) => sender.subscribe(),
        };

        if let Err(err) = receiver.recv().await {
            warn!("Waiting for termination errored: {}", err);
        }
    }

    #[cfg(not(feature = "std"))]
    pub async fn wait_termination(&self) {}

    #[cfg(feature = "metrics")]
    pub(crate) fn get_metrics_readout(&self) -> (Arc<AtomicUsize>, Arc<AtomicUsize>) {
        self.map.get_metrics()
    }
}
