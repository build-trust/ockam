//! Router run state utilities

use crate::messages::NodeMessage;
use ockam_core::{
    compat::collections::{BTreeMap, BTreeSet},
    Address, AddressSet,
};
use tokio::sync::mpsc::Sender;

pub enum NodeState {
    Running,
    Stopping,
}

pub struct RouterState {
    pub(super) sender: Sender<NodeMessage>,
    node_state: NodeState,
    cluster_order: Vec<String>,
    clusters: BTreeMap<String, BTreeSet<Address>>,
}

impl RouterState {
    pub fn new(sender: Sender<NodeMessage>) -> Self {
        Self {
            sender,
            node_state: NodeState::Running,
            cluster_order: vec![],
            clusters: BTreeMap::new(),
        }
    }

    /// Toggle this router to shut down soon
    pub fn shutdown(&mut self) {
        self.node_state = NodeState::Stopping
    }

    /// Check if this router is still `running`, meaning allows
    /// spawning new workers and processors
    pub fn node_state(&self) -> &NodeState {
        &self.node_state
    }

    pub fn set_cluster(&mut self, label: String, addrs: AddressSet) {
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
}
