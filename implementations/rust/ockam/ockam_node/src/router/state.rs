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
}

impl RouterState {
    pub fn new(sender: Sender<NodeMessage>) -> Self {
        Self {
            sender,
            node_state: NodeState::Running,
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
}
