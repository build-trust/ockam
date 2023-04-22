//! This crate provides a UDP Transport for Ockam's Routing Protocol.
//!
//! ## Examples
//
// In `ockam_transport_udp` directory, run an echo server
// with command `cargo run --example echo_server`
//
// Then, run a client that sends a hello message to the server
// with command `cargo run --example client`
use ockam_core::TransportType;

pub use hole_puncher::{PunchError, UdpHolePuncher};
pub use rendezvous_service::UdpRendezvousService;
pub use transport::UdpTransport;
pub use transport::UdpTransportExtension;

mod hole_puncher;
mod rendezvous_service;
mod router;
mod transport;
mod workers;

pub const UDP: TransportType = TransportType::new(2);

pub const CLUSTER_NAME: &str = "_internals.transport.udp";
