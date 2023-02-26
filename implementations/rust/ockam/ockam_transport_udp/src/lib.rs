use ockam_core::TransportType;
pub use transport::*;

mod router;
mod transport;
mod workers;

pub const UDP: TransportType = TransportType::new(2);

pub const CLUSTER_NAME: &str = "_internals.transport.udp";
