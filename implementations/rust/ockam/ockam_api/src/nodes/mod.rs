pub mod config;
pub(crate) mod connection;
pub mod models;
pub mod registry;
pub mod service;
pub use service::background_node::*;

/// A const address to bind and send messages to
pub const NODEMANAGER_ADDR: &str = "_internal.nodemanager";

pub use service::credentials::*;
/// The main node-manager service running on remote nodes
pub use service::{IdentityOverride, NodeManager, NodeManagerWorker};
