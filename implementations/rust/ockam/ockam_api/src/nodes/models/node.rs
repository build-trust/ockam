//! Nodemanager API types

use crate::cli_state::{NodeInfo, NodeProcessStatus};
use minicbor::{Decode, Encode};
use ockam::identity::Identifier;
use serde::{Deserialize, Serialize};

/// Response body for a node status request
#[derive(Debug, Clone, Serialize, Deserialize, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct NodeStatus {
    #[n(1)] pub name: String,
    #[n(2)] pub identifier: Identifier,
    #[n(3)] pub status: NodeProcessStatus,
}

impl NodeStatus {
    pub fn new(name: impl Into<String>, identifier: Identifier, status: NodeProcessStatus) -> Self {
        Self {
            name: name.into(),
            identifier,
            status,
        }
    }
}

impl From<&NodeInfo> for NodeStatus {
    fn from(node: &NodeInfo) -> Self {
        Self {
            name: node.name(),
            identifier: node.identifier(),
            status: node.status(),
        }
    }
}
