mod base;
mod config;
mod forwarder;
mod identity;
mod portal;
mod registry;
mod secure_channel;
pub(crate) mod service;
mod services;
mod transport;
mod vault;

/// Messaging types for the node manager service
///
/// This module is only a type facade and should not have any logic of
/// its own
pub mod types {
    pub use super::base::*;
    pub use super::forwarder::*;
    pub use super::identity::*;
    pub use super::portal::*;
    pub use super::secure_channel::*;
    pub use super::services::*;
    pub use super::transport::*;
    pub use super::vault::*;
}

/// A const address to bind and send messages to
pub const NODEMAN_ADDR: &str = "_internal.nodeman";

/// The main node-manager service running on remote nodes
pub use service::NodeMan;
