use crate::Result;
use miette::miette;
use std::net::{Ipv4Addr, SocketAddr};

/// Helper fn for parsing ip and port from user input
/// It can parse a string containing either an `ip:port` pair or just a `port`
/// into a valid SocketAddr instance.
pub(crate) fn socket_addr_parser(input: &str) -> Result<SocketAddr> {
    let addr: Vec<&str> = input.split(':').collect();
    match addr.len() {
        // Only the port is available
        1 => {
            let port: u16 = addr[0]
                .parse()
                .map_err(|_| miette!("Invalid port number"))?;
            let ip: Ipv4Addr = [127, 0, 0, 1].into();
            Ok(SocketAddr::new(ip.into(), port))
        }
        // Both the ip and port are available
        2 => {
            let port: u16 = addr[1]
                .parse()
                .map_err(|_| miette!("Invalid port number"))?;
            let ip = addr[0]
                .parse::<Ipv4Addr>()
                .map_err(|_| miette!("Invalid IP address"))?;
            Ok(SocketAddr::new(ip.into(), port))
        }
        _ => Err(miette!("Argument {} is an invalid IP Address or Port", input).into()),
    }
}

#[cfg(test)]
mod tests {
    use ockam_core::compat::net::{IpAddr, Ipv4Addr, SocketAddr};

    use crate::util::parsers::socket_addr_parser;

    #[test]
    fn test_parse_bootstrap_server() {
        // Test case 1: only a port is provided
        let input = "9000";
        let result = socket_addr_parser(input);
        assert!(result.is_ok());
        if let Ok(bootstrap_server) = result {
            assert_eq!(
                SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9000),
                bootstrap_server
            );
        }

        // Test case 2: Any 4 octet combination (IPv4) followed by ":" like in "192.168.0.1:9999"
        let input = "192.168.0.1:9999";
        let result = socket_addr_parser(input);
        assert!(result.is_ok());
        if let Ok(bootstrap_server) = result {
            assert_eq!(
                SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1)), 9999),
                bootstrap_server
            );
        }

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
