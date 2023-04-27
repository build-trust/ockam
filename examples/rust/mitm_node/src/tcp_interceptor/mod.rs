mod registry;
mod transport;
pub mod utils;

pub use registry::*;
pub use transport::*;

mod workers;
pub(crate) use workers::*;

pub(crate) const CLUSTER_NAME: &str = "_internals.transport.tcp_mitm";

#[derive(Clone, Copy)]
pub enum Role {
    ReadSource,
    ReadTarget,
}
