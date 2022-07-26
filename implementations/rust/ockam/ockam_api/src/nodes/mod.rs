mod config;
mod registry;

pub(crate) mod service;

pub mod models;

/// A const address to bind and send messages to
pub const NODEMAN_ADDR: &str = "_internal.nodeman";

/// The main node-manager service running on remote nodes
pub use service::NodeMan;
