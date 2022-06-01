//! Node Manager (Node Man, the superhero that we deserve)

use crate::{Context, Message, Result, Routed, Worker};
use serde::{Deserialize, Serialize};

/// Messaging commands sent to node manager
#[derive(Serialize, Deserialize, Message)]
pub enum NodeManMessage {
    /// Query this node for its status
    Status,
}

/// Reply messages from node manager
#[derive(Serialize, Deserialize, Message)]
pub enum NodeManReply {
    /// Reply with node status information
    Status {
        /// Contains the node
        node_name: String,
        /// Current runtime status
        status: String,
        /// Number of registered workers
        workers: u32,
    },
}

/// Node manager provides a messaging API to interact with the current node
pub struct NodeMan {
    node_name: String,
}

impl NodeMan {
    /// Create a new NodeMan with the node name from the ockam CLI
    pub fn new(node_name: String) -> Self {
        Self { node_name }
    }
}

#[crate::worker]
impl Worker for NodeMan {
    type Message = NodeManMessage;
    type Context = Context;

    async fn handle_message(
        &mut self,
        ctx: &mut Context,
        msg: Routed<NodeManMessage>,
    ) -> Result<()> {
        let return_route = msg.return_route();

        match msg.body() {
            NodeManMessage::Status => {
                ctx.send(
                    return_route,
                    NodeManReply::Status {
                        node_name: self.node_name.clone(),
                        status: "[✓]".into(), // TODO: figure out if the current node is "healthy"
                        workers: ctx.list_workers().await?.len() as u32,
                    },
                )
                .await?
            }
        }

        Ok(())
    }
}
