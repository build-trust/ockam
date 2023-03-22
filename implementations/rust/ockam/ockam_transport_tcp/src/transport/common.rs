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

#[cfg(test)]
mod test {
    use crate::transport::common::parse_socket_addr;
    use core::fmt::Debug;
    use ockam_core::{Error, Result};
    use ockam_transport_core::TransportError;

    fn assert_transport_error<T>(result: Result<T>, error: TransportError)
    where
        T: Debug,
    {
        let invalid_address_error: Error = error.into();
        assert_eq!(result.unwrap_err().code(), invalid_address_error.code())
    }

    #[test]
    fn test_parse_socket_address() {
        let result = parse_socket_addr("hostname:port");
        assert!(result.is_err());
        assert_transport_error(result, TransportError::InvalidAddress);

        let result = parse_socket_addr("example.com");
        assert!(result.is_err());
        assert_transport_error(result, TransportError::InvalidAddress);

        let result = parse_socket_addr("example.com:80");
        assert!(result.is_err());
        assert_transport_error(result, TransportError::InvalidAddress);

        let result = parse_socket_addr("127.0.0.1");
        assert!(result.is_err());
        assert_transport_error(result, TransportError::InvalidAddress);

        let result = parse_socket_addr("127.0.0.1:port");
        assert!(result.is_err());
        assert_transport_error(result, TransportError::InvalidAddress);

        let result = parse_socket_addr("127.0.1:80");
        assert!(result.is_err());
        assert_transport_error(result, TransportError::InvalidAddress);

        let result = parse_socket_addr("127.0.0.1:65536");
        assert!(result.is_err());
        assert_transport_error(result, TransportError::InvalidAddress);

        let result = parse_socket_addr("127.0.0.1:0");
        assert!(result.is_ok());

        let result = parse_socket_addr("127.0.0.1:80");
        assert!(result.is_ok());

        let result = parse_socket_addr("127.0.0.1:8080");
        assert!(result.is_ok());
    }
}
