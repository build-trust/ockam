//! [`RemoteForwarder`] allows registering node within a Cloud Node with dynamic or static alias,
//! which allows other nodes forward messages to local workers on this node using that alias.

mod addresses;
mod forwarder;
mod forwarder_worker;
mod info;
mod trust_options;

pub(crate) use addresses::*;
pub use forwarder::*;
pub use forwarder_worker::*;
pub use info::*;
pub use trust_options::*;
