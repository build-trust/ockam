use crate::resolve_peer;
use core::fmt::{Display, Formatter};
use core::str::FromStr;
use minicbor::{Decode, Encode};
use ockam_core::errcode::{Kind, Origin};
use std::net::SocketAddr;

/// Hostname and port
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct HostnamePort {
    #[n(0)]
    hostname: String,
    #[n(1)]
    port: u16,
}

impl HostnamePort {
    /// Create a new HostnamePort
    pub fn new(hostname: &str, port: u16) -> HostnamePort {
        HostnamePort {
            hostname: hostname.to_string(),
            port,
        }
    }

    /// Return a hostname and port from a socket address
    pub fn from_socket_addr(socket_addr: SocketAddr) -> ockam_core::Result<HostnamePort> {
        HostnamePort::from_str(&socket_addr.to_string())
    }

    /// Return a socket address from a hostname and port
    pub fn to_socket_addr(&self) -> ockam_core::Result<SocketAddr> {
        resolve_peer(self.to_string())
    }

    /// Return the hostname
    pub fn hostname(&self) -> String {
        self.hostname.clone()
    }

    /// Return the port
    pub fn port(&self) -> u16 {
        self.port
    }
}

impl FromStr for HostnamePort {
    type Err = ockam_core::Error;

    /// Return a hostname and port when separated by a :
    fn from_str(hostname_port: &str) -> ockam_core::Result<HostnamePort> {
        let mut values = hostname_port.split(':').collect::<Vec<_>>();
        values.reverse();
        let mut values = values.iter();

        match values.next() {
            Some(port) => match port.parse::<u16>().ok() {
                Some(port) => {
                    let hostname = values.map(|v| v.to_string()).rev().collect::<Vec<_>>();
                    let hostname = if hostname.is_empty()
                        || (hostname.len() == 1 && hostname.join("").is_empty())
                    {
                        "127.0.0.1".to_string()
                    } else {
                        hostname.join(":")
                    };
                    Ok(HostnamePort { hostname, port })
                }
                None => Err(ockam_core::Error::new(
                    Origin::Api,
                    Kind::Serialization,
                    format!("cannot read the port as an integer: {port}"),
                ))?,
            },
            _ => Err(ockam_core::Error::new(
                Origin::Api,
                Kind::Serialization,
                format!("cannot read the value as hostname:port {hostname_port}"),
            ))?,
        }
    }
}

impl Display for HostnamePort {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("{}:{}", self.hostname, self.port))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::str::FromStr;

    #[test]
    fn test_hostname_port() -> ockam_core::Result<()> {
        let actual = HostnamePort::from_str("localhost:80")?;
        assert_eq!(actual, HostnamePort::new("localhost", 80));

        let actual = HostnamePort::from_str("127.0.0.1:80")?;
        assert_eq!(actual, HostnamePort::new("127.0.0.1", 80));

        let actual = HostnamePort::from_str(":80")?;
        assert_eq!(actual, HostnamePort::new("127.0.0.1", 80));

        let actual = HostnamePort::from_str("80")?;
        assert_eq!(actual, HostnamePort::new("127.0.0.1", 80));

        let socket_addr = resolve_peer("76.76.21.21:8080".to_string()).unwrap();
        let actual = HostnamePort::from_socket_addr(socket_addr).ok();
        assert_eq!(actual, Some(HostnamePort::new("76.76.21.21", 8080)));

        let actual = HostnamePort::from_str("[2001:db8:85a3::8a2e:370:7334]:8080")?;
        assert_eq!(
            actual,
            HostnamePort::new("[2001:db8:85a3::8a2e:370:7334]", 8080)
        );

        let socket_addr = SocketAddr::from_str("[2001:db8:85a3::8a2e:370:7334]:8080").unwrap();
        let actual = HostnamePort::from_socket_addr(socket_addr).ok();
        assert_eq!(
            actual,
            Some(HostnamePort::new("[2001:db8:85a3::8a2e:370:7334]", 8080))
        );

        Ok(())
    }
}
