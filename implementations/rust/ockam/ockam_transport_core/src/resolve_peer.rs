use crate::TransportError;
use core::net::SocketAddr;
use ockam_core::compat::string::ToString;
use ockam_core::Result;

/// Resolve the given peer to a [`SocketAddr`](std::net::SocketAddr)
#[cfg(feature = "std")]
pub fn resolve_peer(peer: String) -> Result<SocketAddr> {
    // Try to parse as SocketAddr
    if let Ok(p) = parse_socket_addr(&peer) {
        return Ok(p);
    }

    use ockam_core::compat::net::ToSocketAddrs;

    // Try to resolve hostname
    match peer.to_socket_addrs() {
        Ok(mut iter) => {
            // Prefer ip4
            if let Some(p) = iter.find(|x| x.is_ipv4()) {
                return Ok(p);
            }
            if let Some(p) = iter.find(|x| x.is_ipv6()) {
                return Ok(p);
            };
            Err(TransportError::InvalidAddress(format!(
                "cannot resolve address: {peer}. No IP4 or IP6 address found."
            )))?
        }
        Err(e) => Err(TransportError::InvalidAddress(format!(
            "cannot resolve address: {peer}: {e:?}"
        )))?,
    }
}

pub fn parse_socket_addr(s: &str) -> Result<SocketAddr> {
    Ok(s.parse()
        .map_err(|_| TransportError::InvalidAddress(s.to_string()))?)
}

#[cfg(test)]
mod test {
    use crate::{parse_socket_addr, TransportError};
    use core::fmt::Debug;
    use ockam_core::{Error, Result};

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
        assert_transport_error(
            result,
            TransportError::InvalidAddress("hostname:port".to_string()),
        );

        let result = parse_socket_addr("example.com");
        assert!(result.is_err());
        assert_transport_error(
            result,
            TransportError::InvalidAddress("example.com".to_string()),
        );

        let result = parse_socket_addr("example.com:80");
        assert!(result.is_err());
        assert_transport_error(
            result,
            TransportError::InvalidAddress("example.com:80".to_string()),
        );

        let result = parse_socket_addr("127.0.0.1");
        assert!(result.is_err());
        assert_transport_error(
            result,
            TransportError::InvalidAddress("127.0.0.1".to_string()),
        );

        let result = parse_socket_addr("127.0.0.1:port");
        assert!(result.is_err());
        assert_transport_error(
            result,
            TransportError::InvalidAddress("127.0.0.1:port".to_string()),
        );

        let result = parse_socket_addr("127.0.1:80");
        assert!(result.is_err());
        assert_transport_error(
            result,
            TransportError::InvalidAddress("127.0.1:80".to_string()),
        );

        let result = parse_socket_addr("127.0.0.1:65536");
        assert!(result.is_err());
        assert_transport_error(
            result,
            TransportError::InvalidAddress("127.0.0.1:65536".to_string()),
        );

        let result = parse_socket_addr("127.0.0.1:0");
        assert!(result.is_ok());

        let result = parse_socket_addr("127.0.0.1:80");
        assert!(result.is_ok());

        let result = parse_socket_addr("127.0.0.1:8080");
        assert!(result.is_ok());
    }
}
