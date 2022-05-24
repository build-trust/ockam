//! Router run state utilities

use crate::channel_types::SmallSender;
use crate::messages::{NodeMessage, NodeReplyResult};

pub enum NodeState {
    Running,
    Stopping(SmallSender<NodeReplyResult>),
    Dead,
}

pub struct RouterState {
    pub(super) sender: SmallSender<NodeMessage>,
    node_state: NodeState,
}

impl RouterState {
    pub fn new(sender: SmallSender<NodeMessage>) -> Self {
        Self {
            sender,
            node_state: NodeState::Running,
        }
    }

    /// Toggle this router to shut down soon
    pub(super) fn shutdown(&mut self, reply: SmallSender<NodeReplyResult>) {
        self.node_state = NodeState::Stopping(reply)
    }

    /// Ungracefully kill the router
    pub(super) fn kill(&mut self) {
        self.node_state = NodeState::Dead;
    }

    pub(super) fn stop_reply(&self) -> Option<SmallSender<NodeReplyResult>> {
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
