use clap::error::{Error, ErrorKind};
use std::net::SocketAddr;
use std::str::FromStr;
use std::time::Duration;

use miette::miette;

use ockam::identity::Identifier;
use ockam::transport::resolve_peer;
use ockam_api::config::lookup::InternetAddress;
use ockam_core::env::parse_duration;

use crate::util::validators::cloud_resource_name_validator;
use crate::Result;

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

/// Helper fn for parsing an identifier from user input by using
/// [`ockam_identity::Identifier::from_str()`]
pub(crate) fn identity_identifier_parser(input: &str) -> Result<Identifier> {
    Ok(Identifier::from_str(input).map_err(|_| miette!("Invalid identity identifier: {input}"))?)
}

/// Helper fn for parsing an InternetAddress from user input by using
/// [`InternetAddress::new()`]
pub(crate) fn internet_address_parser(input: &str) -> Result<InternetAddress> {
    Ok(InternetAddress::new(input).ok_or_else(|| miette!("Invalid address: {input}"))?)
}

pub(crate) fn project_name_parser(s: &str) -> Result<String> {
    match cloud_resource_name_validator(s) {
        Ok(_) => Ok(s.to_string()),
        Err(_e)=> Err(miette!(
            "project name can contain only alphanumeric characters and the '-', '_' and '.' separators. \
            Separators must occur between alphanumeric characters. This implies that separators can't \
            occur at the start or end of the name, nor they can occur in sequence.",
        ))?,
    }
}

pub(crate) fn duration_parser(arg: &str) -> std::result::Result<Duration, clap::Error> {
    parse_duration(arg).map_err(|_| Error::raw(ErrorKind::InvalidValue, "Invalid duration."))
}

#[cfg(test)]
mod tests {
    use std::net::Ipv6Addr;

    use ockam_core::compat::net::{IpAddr, Ipv4Addr, SocketAddr};

    use super::*;

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
