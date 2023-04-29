use crate::Result;
use anyhow::anyhow;
use std::{
    net::{Ipv4Addr, SocketAddr},
    str::FromStr,
};

use ockam_api::{port_range::PortRange, DefaultAddress};
use ockam_multiaddr::MultiAddr;

pub(crate) mod consumer;
pub(crate) mod producer;

const KAFKA_DEFAULT_PROJECT_ROUTE: &str = "/project/default";
const KAFKA_DEFAULT_CONSUMER_SERVER: &str = "127.0.0.1:4000";
const KAFKA_DEFAULT_CONSUMER_PORT_RANGE: &str = "4001-4100";
const KAFKA_DEFAULT_PRODUCER_SERVER: &str = "127.0.0.1:5000";
const KAFKA_DEFAULT_PRODUCER_PORT_RANGE: &str = "5001-5100";

fn kafka_consumer_default_addr() -> String {
    DefaultAddress::KAFKA_CONSUMER.to_string()
}

fn kafka_producer_default_addr() -> String {
    DefaultAddress::KAFKA_PRODUCER.to_string()
}

fn kafka_default_project_route() -> MultiAddr {
    MultiAddr::from_str(KAFKA_DEFAULT_PROJECT_ROUTE).expect("Failed to parse default project route")
}

fn kafka_default_consumer_server() -> SocketAddr {
    SocketAddr::from_str(KAFKA_DEFAULT_CONSUMER_SERVER)
        .expect("Failed to parse default consumer server")
}

fn kafka_default_consumer_port_range() -> PortRange {
    PortRange::from_str(KAFKA_DEFAULT_CONSUMER_PORT_RANGE)
        .expect("Failed to parse default consumer port range")
}

fn kafka_default_producer_server() -> SocketAddr {
    SocketAddr::from_str(KAFKA_DEFAULT_PRODUCER_SERVER)
        .expect("Failed to parse default producer server")
}

fn kafka_default_producer_port_range() -> PortRange {
    PortRange::from_str(KAFKA_DEFAULT_PRODUCER_PORT_RANGE)
        .expect("Failed to parse default producer port range")
}

/// Helper routine for parsing bootstrap server ip and port from user input
/// It can parse a string containing either an `ip:port` pair or just a `port`
/// into a valid SocketAddr instance.
fn parse_bootstrap_server(bootstrap_server: &str) -> Result<SocketAddr> {
    let addr: Vec<&str> = bootstrap_server.split(':').collect();
    match addr.len() {
        // Only the port is available
        1 => {
            let port: u16 = addr[0]
                .parse()
                .map_err(|_| anyhow!("Invalid port number"))?;
            let ip: Ipv4Addr = [127, 0, 0, 1].into();
            Ok(SocketAddr::new(ip.into(), port))
        }
        // Both the ip and port are available
        2 => {
            let port: u16 = addr[1]
                .parse()
                .map_err(|_| anyhow!("Invalid port number"))?;
            let ip = addr[0]
                .parse::<Ipv4Addr>()
                .map_err(|_| anyhow!("Invalid IP address"))?;
            Ok(SocketAddr::new(ip.into(), port))
        }
        _ => Err(anyhow!("Failed to parse bootstrap server").into()),
    }
}

#[cfg(test)]
mod tests {
    use ockam_core::compat::net::{IpAddr, Ipv4Addr, SocketAddr};

    use crate::kafka::parse_bootstrap_server;

    #[test]
    fn test_parse_bootstrap_server() {
        // Test case 1: only a port is provided
        let input = "9000";
        let result = parse_bootstrap_server(input);
        assert!(result.is_ok());
        if let Ok(bootstrap_server) = result {
            assert_eq!(
                SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9000),
                bootstrap_server
            );
        }

        // Test case 2: Any 4 octet combination (IPv4) followed by ":" like in "192.168.0.1:9999"
        let input = "192.168.0.1:9999";
        let result = parse_bootstrap_server(input);
        assert!(result.is_ok());
        if let Ok(bootstrap_server) = result {
            assert_eq!(
                SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1)), 9999),
                bootstrap_server
            );
        }

        // Test case 3: Any other format will throw an error
        let invalid_input = "invalid";
        assert!(parse_bootstrap_server(invalid_input).is_err());

        let invalid_input = "192.168.0.1:invalid";
        assert!(parse_bootstrap_server(invalid_input).is_err());

        let invalid_input = "192.168.0.1:9999:extra";
        assert!(parse_bootstrap_server(invalid_input).is_err());
        let invalid_input = "192,166,0.1:9999";
        assert!(parse_bootstrap_server(invalid_input).is_err());
    }
}
