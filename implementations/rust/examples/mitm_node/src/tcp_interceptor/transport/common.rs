use ockam_core::compat::net::SocketAddr;
use ockam_core::Result;
use ockam_transport_core::TransportError;

pub(super) fn parse_socket_addr(s: &str) -> Result<SocketAddr> {
    Ok(s.parse().map_err(|_| TransportError::InvalidAddress(s.to_string()))?)
}
