use crate::relay::{CtrlSignal, RelayMessage};
use crate::tokio::sync::mpsc::Sender;
use crate::{
    error::{NodeError, NodeReason},
    NodeReplyResult, RouterReply,
};
use ockam_core::{
    compat::{
        collections::{BTreeMap, BTreeSet},
        string::String,
        vec::Vec,
    },
    Address, AddressSet, Result,
};

/// Address states and associated logic
#[derive(Default)]
pub struct InternalMap {
    /// Registry of primary address to worker address record state
    pub(super) internal: BTreeMap<Address, AddressRecord>,
    /// Alias-registry to map arbitrary address to primary addresses
    pub(super) addr_map: BTreeMap<Address, Address>,
    /// The order in which clusters are allocated and de-allocated
    cluster_order: Vec<String>,
    /// Cluster data records
    clusters: BTreeMap<String, BTreeSet<Address>>,
    /// Track stop information
    stopping: BTreeSet<Address>,
}

impl InternalMap {
    /// Add an address to a particular cluster
    pub(super) fn set_cluster(&mut self, label: String, primary: Address) -> NodeReplyResult {
        let rec = self
            .internal
            .get(&primary)
            .ok_or_else(|| NodeError::Address(primary).not_found())?;

        // If this is the first time we see this cluster ID
        if !self.clusters.contains_key(&label) {
            self.clusters.insert(label.clone(), BTreeSet::new());
            self.cluster_order.push(label.clone());
        }

        // Add all addresses to the cluster set
        for addr in rec.address_set().clone() {
            self.clusters
                .get_mut(&label)
                .expect("No such cluster??")
                .insert(addr);
        }

        RouterReply::ok()
    }

    /// Set an address as ready and return the list of waiting pollers
    pub(super) fn set_ready(&mut self, addr: Address) -> Result<Vec<Sender<NodeReplyResult>>> {
        let addr_record = self
            .internal
            .get_mut(&addr)
            .ok_or_else(|| NodeError::Address(addr).not_found())?;
        Ok(addr_record.set_ready())
    }

    /// Get the ready state of an address
    pub(super) fn get_ready(&mut self, addr: Address, reply: Sender<NodeReplyResult>) -> bool {
        self.internal
            .get_mut(&addr)
            .map_or(false, |rec| rec.ready(reply))
    }

    /// Retrieve the next cluster in reverse-initialsation order
    pub(super) fn next_cluster(&mut self) -> Option<Vec<&mut AddressRecord>> {
        let name = self.cluster_order.pop()?;
        let addrs = self.clusters.remove(&name)?;
        Some(
            self.internal
                .iter_mut()
                .filter_map(|(primary, rec)| {
                    if addrs.contains(primary) {
                        Some(rec)
                    } else {
                        None
                    }
                })
                .collect(),
        )
    }

    /// Mark this address as "having started to stop"
    pub(super) fn init_stop(&mut self, addr: Address) {
        self.stopping.insert(addr);
    }

    /// Check whether the current cluster of addresses was stopped
    pub(super) fn cluster_done(&self) -> bool {
        self.stopping.is_empty()
    }

    /// Get all addresses of workers not in a cluster
    pub(super) fn non_cluster_workers(&mut self) -> Vec<&mut AddressRecord> {
        let clustered = self
            .clusters
            .iter()
            .fold(BTreeSet::new(), |mut acc, (_, set)| {
                acc.append(&mut set.clone());
                acc
            });

        self.internal
            .iter_mut()
            .filter_map(|(addr, rec)| {
                if clustered.contains(addr) {
                    None
                } else {
                    Some(rec)
                }
            })
            // Filter all bare workers because they don't matter
            .filter(|rec| !rec.meta.bare)
            .collect()
    }

    /// Permanently free all remainin resources associated to a particular address
    pub(super) fn free_address(&mut self, primary: Address) {
        self.stopping.remove(&primary);
        if let Some(record) = self.internal.remove(&primary) {
            for addr in record.address_set {
                self.addr_map.remove(&addr);
            }
        }
    }
}

/// Additional metadata for address records
#[derive(Debug)]
pub struct AddressMeta {
    pub processor: bool,
    pub bare: bool,
}

#[derive(Debug)]
pub struct AddressRecord {
    address_set: AddressSet,
    sender: Option<Sender<RelayMessage>>,
    ctrl_tx: Sender<CtrlSignal>,
    state: AddressState,
    ready: ReadyState,
    meta: AddressMeta,
}

impl AddressRecord {
    pub fn address_set(&self) -> &AddressSet {
        &self.address_set
    }
    pub fn sender(&self) -> Sender<RelayMessage> {
        self.sender.clone().expect("No such sender!")
    }
    pub fn sender_drop(&mut self) {
        self.sender = None;
    }
    pub fn new(
        address_set: AddressSet,
        sender: Sender<RelayMessage>,
        ctrl_tx: Sender<CtrlSignal>,
        meta: AddressMeta,
    ) -> Self {
        AddressRecord {
            address_set,
            sender: Some(sender),
            ctrl_tx,
            state: AddressState::Running,
            ready: ReadyState::Initialising(vec![]),
            meta,
        }
    }

    /// Signal this worker to stop -- it will no longer be able to receive messages
    pub async fn stop(&mut self) -> Result<()> {
        if self.meta.processor {
            self.ctrl_tx
                .send(CtrlSignal::InterruptStop)
                .await
                .map_err(|_| NodeError::NodeState(NodeReason::Unknown).internal())?;
        } else {
            self.sender = None;
        }
        self.state = AddressState::Stopping;
        Ok(())
    }

    /// Check the integrity of this record
    pub fn check(&self) -> bool {
        self.state == AddressState::Running
    }

    /// Check whether this address has been marked as ready yet and if
    /// it hasn't we register our sender for future notification
    pub fn ready(&mut self, reply: Sender<NodeReplyResult>) -> bool {
        match self.ready {
            ReadyState::Ready => true,
            ReadyState::Initialising(ref mut vec) => {
                vec.push(reply);
                false
            }
        }
    }

    /// Mark this address as 'ready' and return the list of active pollers
    pub fn set_ready(&mut self) -> Vec<Sender<NodeReplyResult>> {
        let waiting = core::mem::replace(&mut self.ready, ReadyState::Ready);
        match waiting {
            ReadyState::Initialising(vec) => vec,
            ReadyState::Ready => vec![],
        }
    }
}

/// Encode the run states a worker or processor can be in
#[derive(Debug, PartialEq)]
pub enum AddressState {
    /// The runner is looping in its main body (either handling messages or a manual run-loop)
    Running,
    /// The runner was signalled to shut-down (running `shutdown()`)
    Stopping,
    /// The runner has experienced an error and is waiting for supervisor intervention
    #[allow(unused)]
    Faulty,
}

/// Encode the ready state of a worker or processor
#[derive(Debug)]
pub enum ReadyState {
    /// THe runner is fully ready
    Ready,
    /// The runner is still processing user init code and contains a list of waiting polling addresses
    Initialising(Vec<Sender<NodeReplyResult>>),
}
