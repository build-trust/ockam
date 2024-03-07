pub(crate) mod connection;
pub mod models;
pub mod registry;
pub mod service;

pub use service::background_node_client::*;
pub use service::in_memory_node::*;
pub use service::policy::*;
/// The main node-manager service running on remote nodes
pub use service::{NodeManager, NodeManagerWorker};

/// A const address to bind and send messages to
pub const NODEMANAGER_ADDR: &str = "_internal.nodemanager";
