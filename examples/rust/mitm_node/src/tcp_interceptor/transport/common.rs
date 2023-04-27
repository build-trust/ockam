use ockam_core::compat::net::{SocketAddr, ToSocketAddrs};
use ockam_core::Result;
use ockam_transport_core::TransportError;

/// Resolve the given peer to a [`SocketAddr`](std::net::SocketAddr)
pub(super) fn resolve_peer(peer: String) -> Result<SocketAddr> {
    // Try to parse as SocketAddr
    if let Ok(p) = parse_socket_addr(&peer) {
        return Ok(p);
    }

    // Try to resolve hostname
    if let Ok(mut iter) = peer.to_socket_addrs() {
        // Prefer ip4
        if let Some(p) = iter.find(|x| x.is_ipv4()) {
            return Ok(p);
        }
        if let Some(p) = iter.find(|x| x.is_ipv6()) {
            return Ok(p);
        }
    }

    // Nothing worked, return an error
    Err(TransportError::InvalidAddress.into())
}

pub(super) fn parse_socket_addr(s: &str) -> Result<SocketAddr> {
    Ok(s.parse().map_err(|_| TransportError::InvalidAddress)?)
}
