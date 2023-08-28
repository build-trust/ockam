use crate::Result;
use miette::miette;
use ockam_transport_tcp::resolve_peer;
use std::net::SocketAddr;

/// Helper function for parsing a socket from user input
/// It is possible to just input a `port`. In that case the address will be assumed to be
/// 127.0.0.1:<port>
pub(crate) fn socket_addr_parser(input: &str) -> Result<SocketAddr> {
    let addr: Vec<&str> = input.split(':').collect();

    let address = match addr.len() {
        // Only the port is available
        1 => format!("127.0.0.1:{}", addr[0]),
        // Both the ip and port are available
        _ => input.to_string(),
    };
    Ok(resolve_peer(address.to_string())
        .map_err(|e| miette!("cannot parse the address {address} as a socket address: {e}"))?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ockam_core::compat::net::{IpAddr, Ipv4Addr, SocketAddr};
    use std::net::Ipv6Addr;

    #[test]
    fn test_parse_port_only() {
        let input = "9000";
        let result = socket_addr_parser(input);
        assert!(result.is_ok());
        assert_eq!(
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9000),
            result.unwrap()
        );
    }

    #[test]
    fn test_ipv4_and_port() {
        let input = "192.168.0.1:9999";
        let result = socket_addr_parser(input);
        assert!(result.is_ok());
        assert_eq!(
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1)), 9999),
            result.unwrap()
        );
    }

    #[test]
    fn test_ipv6_and_port() {
        let input = "[::1]:9999";
        let result = socket_addr_parser(input);
        assert!(result.is_ok());
        assert_eq!(
            SocketAddr::new(IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)), 9999),
            result.unwrap()
        );
    }

    #[test]
    fn test_localhost() {
        let input = "localhost:9999";
        let result = socket_addr_parser(input);
        assert!(result.is_ok());
        assert!(result.is_ok());
        assert_eq!(
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9999),
            result.unwrap()
        );
    }

    #[test]
    fn test_invalid_inputs() {
        // Test case 3: Any other format will throw an error
        let invalid_input = "invalid";
        assert!(socket_addr_parser(invalid_input).is_err());

        let invalid_input = "192.168.0.1:invalid";
        assert!(socket_addr_parser(invalid_input).is_err());

        let invalid_input = "192.168.0.1:9999:extra";
        assert!(socket_addr_parser(invalid_input).is_err());
        let invalid_input = "192,166,0.1:9999";
        assert!(socket_addr_parser(invalid_input).is_err());
    }
}
