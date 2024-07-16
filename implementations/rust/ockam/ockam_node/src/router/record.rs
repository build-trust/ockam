use crate::channel_types::{MessageSender, SmallSender};
use crate::relay::CtrlSignal;
use crate::{
    error::{NodeError, NodeReason},
    NodeReplyResult, RouterReply,
};
use core::sync::atomic::{AtomicUsize, Ordering};
use ockam_core::{
    compat::{
        collections::{BTreeMap, BTreeSet},
        string::String,
        sync::Arc,
        vec::Vec,
    },
    flow_control::FlowControls,
    Address, AddressAndMetadata, AddressMetadata, RelayMessage, Result,
};

/// Address states and associated logic
pub struct InternalMap {
    /// Registry of primary address to worker address record state
    address_records_map: BTreeMap<Address, AddressRecord>,
    /// Alias-registry to map arbitrary address to primary addresses
    alias_map: BTreeMap<Address, Address>,
    /// Registry of arbitrary metadata for each address, lazily populated
    address_metadata_map: BTreeMap<Address, AddressMetadata>,
    /// The order in which clusters are allocated and de-allocated
    cluster_order: Vec<String>,
    /// Cluster data records
    clusters: BTreeMap<String, BTreeSet<Address>>,
    /// Track stop information for Clusters
    stopping: BTreeSet<Address>,
    /// Access to [`FlowControls`] to clean resources
    flow_controls: FlowControls,
    /// Metrics collection and sharing
    #[cfg(feature = "metrics")]
    metrics: (Arc<AtomicUsize>, Arc<AtomicUsize>),
}

impl InternalMap {
    pub(super) fn new(flow_controls: &FlowControls) -> Self {
        Self {
            address_records_map: Default::default(),
            alias_map: Default::default(),
            address_metadata_map: Default::default(),
            cluster_order: Default::default(),
            clusters: Default::default(),
            stopping: Default::default(),
            flow_controls: flow_controls.clone(),
            #[cfg(feature = "metrics")]
            metrics: Default::default(),
        }
    }
}

impl InternalMap {
    pub(super) fn clear_address_records_map(&mut self) {
        self.address_records_map.clear()
    }

    pub(super) fn get_address_record(&self, primary_address: &Address) -> Option<&AddressRecord> {
        self.address_records_map.get(primary_address)
    }

    pub(super) fn get_address_record_mut(
        &mut self,
        primary_address: &Address,
    ) -> Option<&mut AddressRecord> {
        self.address_records_map.get_mut(primary_address)
    }

    pub(super) fn address_records_map(&self) -> &BTreeMap<Address, AddressRecord> {
        &self.address_records_map
    }

    pub(super) fn remove_address_record(
        &mut self,
        primary_address: &Address,
    ) -> Option<AddressRecord> {
        self.flow_controls.cleanup_address(primary_address);
        self.address_records_map.remove(primary_address)
    }

    pub(super) fn insert_address_record(
        &mut self,
        primary_address: Address,
        record: AddressRecord,
    ) -> Option<AddressRecord> {
        self.address_records_map.insert(primary_address, record)
    }

    pub(super) fn find_terminal_address(
        &self,
        addresses: &[Address],
    ) -> Option<AddressAndMetadata> {
        addresses.iter().find_map(|address| {
            self.address_metadata_map
                .get(address)
                .filter(|&meta| meta.is_terminal)
                .map(|meta| AddressAndMetadata {
                    address: address.clone(),
                    metadata: meta.clone(),
                })
        })
    }

    pub(super) fn set_address_metadata(&mut self, meta: AddressAndMetadata) {
        let metadata = self.address_metadata_map.entry(meta.address).or_default();
        *metadata = meta.metadata;
    }

    pub(super) fn get_address_metadata(&self, address: &Address) -> Option<AddressMetadata> {
        self.address_metadata_map.get(address).cloned()
    }

    pub(super) fn remove_alias(&mut self, alias_address: &Address) -> Option<Address> {
        self.alias_map.remove(alias_address)
    }

    pub(super) fn insert_alias(&mut self, alias_address: &Address, primary_address: &Address) {
        _ = self
            .alias_map
            .insert(alias_address.clone(), primary_address.clone())
    }

    pub(super) fn get_primary_address(&self, alias_address: &Address) -> Option<&Address> {
        self.alias_map.get(alias_address)
    }
}

impl InternalMap {
    #[cfg(feature = "metrics")]
    pub(super) fn update_metrics(&self) {
        self.metrics
            .0
            .store(self.address_records_map.len(), Ordering::Release);
        self.metrics.1.store(self.clusters.len(), Ordering::Release);
    }

    #[cfg(feature = "metrics")]
    pub(super) fn get_metrics(&self) -> (Arc<AtomicUsize>, Arc<AtomicUsize>) {
        (Arc::clone(&self.metrics.0), Arc::clone(&self.metrics.1))
    }

    #[cfg(feature = "metrics")]
    pub(super) fn get_addr_count(&self) -> usize {
        self.metrics.0.load(Ordering::Acquire)
    }

    /// Add an address to a particular cluster
    pub(super) fn set_cluster(&mut self, label: String, primary: Address) -> NodeReplyResult {
        let rec = self
            .address_records_map
            .get(&primary)
            .ok_or_else(|| NodeError::Address(primary).not_found())?;

        // If this is the first time we see this cluster ID
        if !self.clusters.contains_key(&label) {
            self.clusters.insert(label.clone(), BTreeSet::new());
            self.cluster_order.push(label.clone());
        }

        // Add all addresses to the cluster set
        for addr in rec.address_set() {
            self.clusters
                .get_mut(&label)
                .expect("No such cluster??")
                .insert(addr.clone());
        }

        RouterReply::ok()
    }

    /// Set an address as ready and return the list of waiting pollers
    pub(super) fn set_ready(&mut self, addr: Address) -> Result<Vec<SmallSender<NodeReplyResult>>> {
        let addr_record = self
            .address_records_map
            .get_mut(&addr)
            .ok_or_else(|| NodeError::Address(addr).not_found())?;
        Ok(addr_record.set_ready())
    }

    /// Get the ready state of an address
    pub(super) fn get_ready(&mut self, addr: Address, reply: SmallSender<NodeReplyResult>) -> bool {
        self.address_records_map
            .get_mut(&addr)
            .map_or(false, |rec| rec.ready(reply))
    }

    /// Retrieve the next cluster in reverse-initialisation order
    /// Return None if there is no next cluster or if the cluster
    /// contained no more active addresses
    pub(super) fn next_cluster(&mut self) -> Option<Vec<Address>> {
        // loop until either:
        //  - there are no more clusters
        //  - we found a non-empty list of active addresses in a cluster
        loop {
            let name = self.cluster_order.pop()?;
            let addrs = self.clusters.remove(&name)?;
            let active_addresses: Vec<Address> = self
                .address_records_map
                .iter()
                .filter_map(|(primary, _)| {
                    if addrs.contains(primary) {
                        Some(primary.clone())
                    } else {
                        None
                    }
                })
                .collect();
            if active_addresses.is_empty() {
                continue;
            } else {
                return Some(active_addresses);
            }
        }
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

        self.address_records_map
            .iter_mut()
            .filter_map(|(addr, rec)| {
                if clustered.contains(addr) {
                    None
                } else {
                    Some(rec)
                }
            })
            // Filter all detached workers because they don't matter :(
            .filter(|rec| !rec.meta.detached)
            .collect()
    }

    /// Permanently free all remaining resources associated to a particular address
    pub(super) fn free_address(&mut self, primary: Address) {
        self.stopping.remove(&primary);
        if let Some(record) = self.remove_address_record(&primary) {
            for addr in record.address_set {
                self.remove_alias(&addr);
                self.address_metadata_map.remove(&addr);
            }
        }
    }
}

/// Additional metadata for worker records
#[derive(Debug)]
pub struct WorkerMeta {
    pub processor: bool,
    pub detached: bool,
}

#[derive(Debug)]
pub struct AddressRecord {
    address_set: Vec<Address>,
    sender: Option<MessageSender<RelayMessage>>,
    ctrl_tx: SmallSender<CtrlSignal>, // Unused for not-detached workers
    state: AddressState,
    ready: ReadyState,
    meta: WorkerMeta,
    msg_count: Arc<AtomicUsize>,
}

impl AddressRecord {
    pub fn address_set(&self) -> &[Address] {
        &self.address_set
    }

    pub fn sender(&self) -> MessageSender<RelayMessage> {
        self.sender.clone().expect("No such sender!")
    }

    pub fn drop_sender(&mut self) {
        self.sender = None;
    }

    pub fn new(
        address_set: Vec<Address>,
        sender: MessageSender<RelayMessage>,
        ctrl_tx: SmallSender<CtrlSignal>,
        msg_count: Arc<AtomicUsize>,
        meta: WorkerMeta,
    ) -> Self {
        AddressRecord {
            address_set,
            sender: Some(sender),
            ctrl_tx,
            state: AddressState::Running,
            ready: ReadyState::Initialising(vec![]),
            msg_count,
            meta,
        }
    }

    #[inline]
    pub fn increment_msg_count(&self) {
        self.msg_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Signal this worker to stop -- it will no longer be able to receive messages
    pub async fn stop(&mut self) -> Result<()> {
        if self.meta.processor {
            self.ctrl_tx
                .send(CtrlSignal::InterruptStop)
                .await
                .map_err(|_| NodeError::NodeState(NodeReason::Unknown).internal())?;
        } else {
            self.drop_sender();
        }
        self.state = AddressState::Stopping;
        Ok(())
    }

    /// Check the integrity of this record
    #[inline]
    pub fn check(&self) -> bool {
        self.state == AddressState::Running && self.sender.is_some()
    }

    /// Check whether this address has been marked as ready yet and if
    /// it hasn't we register our sender for future notification
    pub fn ready(&mut self, reply: SmallSender<NodeReplyResult>) -> bool {
        match self.ready {
            ReadyState::Ready => true,
            ReadyState::Initialising(ref mut vec) => {
                vec.push(reply);
                false
            }
        }
    }

    /// Mark this address as 'ready' and return the list of active pollers
    pub fn set_ready(&mut self) -> Vec<SmallSender<NodeReplyResult>> {
        let waiting = core::mem::replace(&mut self.ready, ReadyState::Ready);
        match waiting {
            ReadyState::Initialising(vec) => vec,
            ReadyState::Ready => vec![],
        }
    }
}

/// Encode the run states a worker or processor can be in
#[derive(Debug, PartialEq, Eq)]
pub enum AddressState {
    /// The runner is looping in its main body (either handling messages or a manual run-loop)
    Running,
    /// The runner was signaled to shut-down (running `shutdown()`)
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
    Initialising(Vec<SmallSender<NodeReplyResult>>),
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::channel_types::small_channel;
    use crate::router::record::InternalMap;

    #[test]
    fn test_next_cluster() {
        let mut map = InternalMap::new(&FlowControls::new());

        // create 3 clusters
        //   cluster 1 has one active address
        //   cluster 2 has no active address
        //   cluster 3 has one active address
        map.address_records_map
            .insert("address1".into(), create_address_record("address1"));
        let _ = map.set_cluster("CLUSTER1".into(), "address1".into());

        map.address_records_map
            .insert("address2".into(), create_address_record("address2"));
        let _ = map.set_cluster("CLUSTER2".into(), "address2".into());
        map.free_address("address2".into());

        map.address_records_map
            .insert("address3".into(), create_address_record("address3"));
        let _ = map.set_cluster("CLUSTER3".into(), "address3".into());

        // get the active addresses for cluster 3, clusters are popped in reverse order
        assert_eq!(map.next_cluster(), Some(vec!["address3".into()]));
        // get the active addresses for cluster 1, cluster 2 is skipped
        assert_eq!(map.next_cluster(), Some(vec!["address1".into()]));
        // finally there are no more clusters with active addresses
        assert_eq!(map.next_cluster(), None);
    }

    /// HELPERS
    fn create_address_record(primary: &str) -> AddressRecord {
        let (tx1, _) = small_channel();
        let (tx2, _) = small_channel();
        AddressRecord::new(
            vec![primary.into()],
            tx1,
            tx2,
            Arc::new(AtomicUsize::new(1)),
            WorkerMeta {
                processor: false,
                detached: false,
            },
        )
    }
}
