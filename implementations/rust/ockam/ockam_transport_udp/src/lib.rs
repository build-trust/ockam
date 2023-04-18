use ockam_core::TransportType;

pub use rendezvous_service::UdpRendezvousService;
pub use transport::UdpTransport;
pub use transport::UdpTransportExtension;

mod rendezvous_service;
mod router;
mod transport;
mod workers;

pub const UDP: TransportType = TransportType::new(2);

pub const CLUSTER_NAME: &str = "_internals.transport.udp";
