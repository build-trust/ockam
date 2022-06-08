//! NodeManager response messages

use crate::Message;
use serde::{Deserialize, Serialize};

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
        /// Current pid
        pid: i32,
    },
}
