//! Nodemanager API types

use minicbor::{Decode, Encode};

///////////////////-!  RESPONSE BODIES

/// Response body for a node status
#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct NodeStatus {
    #[n(1)] pub node_name: String,
    #[n(2)] pub status: String,
    #[n(3)] pub workers: u32,
    #[n(4)] pub pid: i32,
}

impl NodeStatus {
    pub fn new(
        node_name: impl Into<String>,
        status: impl Into<String>,
        workers: u32,
        pid: i32,
    ) -> Self {
        Self {
            node_name: node_name.into(),
            status: status.into(),
            workers,
            pid,
        }
    }
}
