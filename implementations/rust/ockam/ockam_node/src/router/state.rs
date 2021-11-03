//! Router run state utilities

use crate::messages::{NodeMessage, NodeReplyResult};
use crate::tokio::sync::mpsc::Sender;

pub enum NodeState {
    Running,
    Stopping(Sender<NodeReplyResult>),
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
    pub(super) fn shutdown(&mut self, reply: Sender<NodeReplyResult>) {
        self.node_state = NodeState::Stopping(reply)
    }

    pub(super) fn stop_reply(&self) -> Option<Sender<NodeReplyResult>> {
        match &self.node_state {
            NodeState::Stopping(sender) => Some(sender.clone()),
            _ => None,
        }
    }

    pub fn running(&self) -> bool {
        core::matches!(self.node_state, NodeState::Running)
    }

    /// Check if this router is still `running`, meaning allows
    /// spawning new workers and processors
    pub fn node_state(&self) -> &NodeState {
        &self.node_state
    }
}
