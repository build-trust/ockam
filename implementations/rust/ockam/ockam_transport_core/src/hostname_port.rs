use crate::parse_socket_addr;
use core::fmt::{Display, Formatter};
use core::net::IpAddr;
use core::net::SocketAddr;
use core::str::FromStr;
use minicbor::{CborLen, Decode, Encode};
use ockam_core::compat::format;
use ockam_core::compat::string::{String, ToString};
use ockam_core::errcode::{Kind, Origin};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// [`HostnamePort`]'s static counterpart usable for const values.
pub struct StaticHostnamePort {
    hostname: &'static str,
    port: u16,
}

impl StaticHostnamePort {
    pub const fn new(hostname: &'static str, port: u16) -> Self {
        Self { hostname, port }
    }
}

impl From<StaticHostnamePort> for HostnamePort {
    fn from(value: StaticHostnamePort) -> Self {
        Self::new(value.hostname, value.port)
    }
}

/// Hostname and port
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, CborLen)]
#[rustfmt::skip]
pub struct HostnamePort {
    #[n(0)] hostname: String,
    #[n(1)] port: u16,
}

impl HostnamePort {
    /// Create a new HostnamePort
    pub fn new(hostname: impl Into<String>, port: u16) -> HostnamePort {
        Self {
            hostname: hostname.into(),
            port,
        }
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

impl From<SocketAddr> for HostnamePort {
    fn from(socket_addr: SocketAddr) -> Self {
        let ip = match socket_addr.ip() {
            IpAddr::V4(ip) => ip.to_string(),
            IpAddr::V6(ip) => format!("[{ip}]"),
        };
        Self {
            hostname: ip,
            port: socket_addr.port(),
        }
    }
}

impl Serialize for HostnamePort {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for HostnamePort {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        HostnamePort::from_str(&s).map_err(serde::de::Error::custom)
    }
}

impl TryFrom<String> for HostnamePort {
    type Error = ockam_core::Error;

    fn try_from(value: String) -> ockam_core::Result<Self> {
        FromStr::from_str(value.as_str())
    }
}

impl TryFrom<&str> for HostnamePort {
    type Error = ockam_core::Error;

    fn try_from(value: &str) -> ockam_core::Result<Self> {
        FromStr::from_str(value)
    }
}

impl FromStr for HostnamePort {
    type Err = ockam_core::Error;

    /// Return a hostname and port when separated by a :
    fn from_str(hostname_port: &str) -> ockam_core::Result<HostnamePort> {
        // edge case: only the port is given
        if let Ok(port) = hostname_port.parse::<u16>() {
            return Ok(HostnamePort::new("127.0.0.1", port));
        }

        if let Some(port_str) = hostname_port.strip_prefix(':') {
            if let Ok(port) = port_str.parse::<u16>() {
                return Ok(HostnamePort::new("127.0.0.1", port));
            }
        }

        if let Ok(socket) = parse_socket_addr(hostname_port) {
            return Ok(HostnamePort::from(socket));
        }

        // We now know it's not an ip, let's validate if it can be a valid hostname
        if let Some((hostname, port_str)) = hostname_port.split_once(':') {
            let port = match port_str.parse::<u16>() {
                Ok(port) => port,
                Err(_) => {
                    return Err(ockam_core::Error::new(
                        Origin::Api,
                        Kind::Serialization,
                        format!("invalid port value: {hostname_port}"),
                    ))
                }
            };

            if !hostname.is_ascii() {
                return Err(ockam_core::Error::new(
                    Origin::Api,
                    Kind::Serialization,
                    format!("hostname must be ascii: {hostname_port}"),
                ));
            }

            // TODO: Validate hostname better
            if hostname
                .as_bytes()
                .iter()
                .any(|c| !c.is_ascii_alphanumeric() && *c != 0x2d && *c != 0x2e)
            {
                return Err(ockam_core::Error::new(
                    Origin::Api,
                    Kind::Serialization,
                    format!("hostname has unsupported bytes: {hostname_port}"),
                ));
            }

            return Ok(HostnamePort::new(hostname, port));
        }

        Err(ockam_core::Error::new(
            Origin::Api,
            Kind::Serialization,
            format!("cannot read the value as hostname:port: {hostname_port}"),
        ))
    }
}

impl Display for HostnamePort {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
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

        // this is malformed address
        let actual = HostnamePort::from_str("127.0.0.1:80:80").ok();
        assert_eq!(actual, None);

        let actual = HostnamePort::from_str(":80")?;
        assert_eq!(actual, HostnamePort::new("127.0.0.1", 80));

        let actual = HostnamePort::from_str("80")?;
        assert_eq!(actual, HostnamePort::new("127.0.0.1", 80));

        let actual = HostnamePort::from_str("[2001:db8:85a3::8a2e:370:7334]:8080")?;
        assert_eq!(
            actual,
            HostnamePort::new("[2001:db8:85a3::8a2e:370:7334]", 8080)
        );

        let socket_addr = SocketAddr::from_str("[2001:db8:85a3::8a2e:370:7334]:8080").unwrap();
        let actual = HostnamePort::from(socket_addr);
        assert_eq!(
            actual,
            HostnamePort::new("[2001:db8:85a3::8a2e:370:7334]", 8080)
        );

        let socket_addr = SocketAddr::from_str("[::1]:8080").unwrap();
        let actual = HostnamePort::from(socket_addr);
        assert_eq!(actual, HostnamePort::new("[::1]", 8080));
        assert_eq!(actual.to_string(), "[::1]:8080");

        let hostname_port = HostnamePort::from_str("xn--74h.com:80").unwrap();
        assert_eq!(hostname_port.hostname(), "xn--74h.com");
        assert_eq!(hostname_port.port(), 80);

        Ok(())
    }

    #[test]
    fn test_invalid_inputs() {
        // we only validate the port, not the hostname
        assert!(HostnamePort::from_str("invalid").is_err());
        assert!(HostnamePort::from_str("192.168.0.1:invalid").is_err());
        assert!(HostnamePort::from_str("192.168.0.1:9999:extra").is_err());
        assert!(HostnamePort::from_str("192,166,0.1:9999").is_err());
    }

    #[test]
    fn test_ipv6_and_port() {
        let actual = HostnamePort::from_str("[::1]:9999").unwrap();
        assert_eq!(actual, HostnamePort::new("[::1]", 9999));
    }
}
