use crate::{exitcode::SOFTWARE, Result};
use miette::miette;
use std::net::SocketAddr;
use url::Host;

/// Helper fn for parsing ip and port from user input
/// It can parse a string containing either an `ip:port` pair or just a `port`
/// into a valid SocketAddr instance. Uses the host_parser fn to initially parse
/// the input as a HostPort and further validates non-Domain type hosts.
pub(crate) fn socket_addr_parser(input: &str) -> Result<SocketAddr> {
    host_parser(input).and_then(|hp| hp.try_into())
}

/// Helper fn for parsing command user inputs and validating for Ipv4 or Domain
/// hosts with a provided port, e.g. `host:port` or just a `port`.
pub(crate) fn host_parser(input: &str) -> Result<HostPort> {
    let addr: Vec<&str> = input.split(':').collect();

    match addr.len() {
        // Only the port is available
        1 => {
            let port: u16 = addr[0]
                .parse()
                .map_err(|_| miette!("Invalid port number"))?;
            let host = Host::Ipv4([127, 0, 0, 1].into());
            Ok(HostPort { host, port })
        }
        // Both the ip and port are available
        2 => {
            let port: u16 = addr[1]
                .parse()
                .map_err(|_| miette!("Invalid port number"))?;
            let host = Host::parse(addr[0])
                .map_err(|_| miette!("Invalid IP address or hostname"))
                .and_then(|value| match value {
                    Host::Ipv6(_) => Err(miette!("Ipv6 is not supported")),
                    _ => Ok(value),
                })?;

            Ok(HostPort { host, port })
        }
        _ => Err(miette!("Argument {} is an invalid IP Address or Port", input).into()),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct HostPort {
    host: Host,
    port: u16,
}

impl TryFrom<HostPort> for SocketAddr {
    type Error = crate::Error;

    fn try_from(value: HostPort) -> std::result::Result<Self, Self::Error> {
        match value.host {
            Host::Domain(_) => Err(Self::Error::new(
                SOFTWARE,
                // DNS resolution occurs at the node
                miette!("Domains can not be converted to SocketAddrs"),
            )),
            Host::Ipv6(_) => Err(Self::Error::new(
                SOFTWARE,
                miette!("Ipv6 addresses are not supported"),
            )),
            Host::Ipv4(addr) => Ok(SocketAddr::new(addr.into(), value.port)),
        }
    }
}

impl std::fmt::Display for HostPort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.host, self.port)
    }
}

#[cfg(test)]
mod tests {
    use ockam_core::compat::net::{IpAddr, Ipv4Addr, SocketAddr};

    use crate::util::parsers::*;

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

    #[test]
    fn test_host_parser() {
        let input = "localhost:9000";

        let result = host_parser(input);
        assert!(result.is_ok());

        if let Ok(host_port) = result {
            assert_eq!(
                HostPort {
                    host: Host::Domain("localhost".into()),
                    port: 9000
                },
                host_port
            );
        }
    }
}
