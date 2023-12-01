//! Nodemanager API types
use crate::nodes::{NodeManager, NodeManagerWorker};
use minicbor::{Decode, Encode};
use ockam_core::api::{Error, RequestHeader, Response};
use ockam_core::Result;
use ockam_node::Context;
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
impl NodeManagerWorker {
    pub async fn get_node_status(
        &self,
        ctx: &Context,
        req: &RequestHeader,
    ) -> Result<Response<NodeStatus>, Response<Error>> {
        let node_name = self.node_manager.get_node_name().await;
        let list_workers_length = ctx.list_workers().await.unwrap().len() as u32;
        Ok(Response::ok(req).body(NodeStatus::new(
            node_name,
            "Running",
            list_workers_length,
            std::process::id() as i32,
        )))
    }
}

impl NodeManager {
    pub async fn get_node_name(&self) -> String {
        self.node_name.clone()
    }

    /// Delete the cli state related to the current node when launched in-memory
    pub async fn delete_node(&self) -> Result<()> {
        self.cli_state.remove_node(&self.node_name).await?;
        Ok(())
    }
}
