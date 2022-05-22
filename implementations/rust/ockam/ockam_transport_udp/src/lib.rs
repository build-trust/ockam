use std::net::SocketAddr;

use ockam_core::{Result, TransportType};
use ockam_transport_core::TransportError;
pub use transport::*;

mod router;
mod transport;
mod workers;

pub const UDP: TransportType = TransportType::new(2);

pub const CLUSTER_NAME: &str = "_internals.transport.udp";

fn parse_socket_addr<S: AsRef<str>>(s: S) -> Result<SocketAddr> {
    Ok(s.as_ref()
        .parse()
        .map_err(|_| TransportError::InvalidAddress)?)
}
