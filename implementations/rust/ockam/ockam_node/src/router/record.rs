use crate::relay::RelayMessage;
use crate::tokio::sync::mpsc::Sender;
use ockam_core::{
    compat::collections::{BTreeMap, BTreeSet},
    Address, AddressSet,
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
}

impl InternalMap {
    pub(super) fn set_cluster(&mut self, label: String, addrs: AddressSet) {
        // If this is the first time we see this cluster ID
        if !self.clusters.contains_key(&label) {
            self.clusters.insert(label.clone(), BTreeSet::new());
            self.cluster_order.push(label.clone());
        }

        // Add all addresses to the cluster set
        for addr in addrs {
            self.clusters.get_mut(&label).unwrap().insert(addr);
        }
    }

    /// Retrieve the next cluster in reverse-initialsation order
    pub(super) fn next_cluster(&mut self) -> Option<BTreeSet<Address>> {
        let name = self.cluster_order.pop()?;
        self.clusters.remove(&name)
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
            .collect()
    }
}

#[derive(Debug)]
pub struct AddressRecord {
    address_set: AddressSet,
    sender: Sender<RelayMessage>,
}

impl AddressRecord {
    pub fn address_set(&self) -> &AddressSet {
        &self.address_set
    }
    pub fn sender(&self) -> Sender<RelayMessage> {
        self.sender.clone()
    }
}

impl AddressRecord {
    pub fn new(address_set: AddressSet, sender: Sender<RelayMessage>) -> Self {
        AddressRecord {
            address_set,
            sender,
        }
    }
}

/// Encode the run states a worker or processor can be in
///
/// * Running - the runner is looping in its main body (either
///   handling messages or a manual run-loop)
/// * Stopping - the runner was signalled to shut-down (running `shutdown()`)
/// * Faulty - the runner has experienced an error and is waiting for
///   supervisor intervention
#[derive(Debug)]
#[allow(unused)]
pub enum AddressState {
    Running,
    Stopping,
    Faulty,
}
