pub(crate) mod connection;
pub mod models;
pub mod registry;
pub mod service;

pub use service::background_node::*;
pub use service::credentials::*;
pub use service::in_memory_node::*;
/// The main node-manager service running on remote nodes
pub use service::{IdentityOverride, NodeManager, NodeManagerWorker};

/// A const address to bind and send messages to
pub const NODEMANAGER_ADDR: &str = "_internal.nodemanager";
