use core::fmt::{Display, Formatter};
use core::net::IpAddr;
use core::net::SocketAddr;
use core::str::FromStr;
use minicbor::{CborLen, Decode, Encode};
use ockam_core::compat::format;
use ockam_core::compat::string::{String, ToString};
use ockam_core::errcode::{Kind, Origin};

/// Hostname and port
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, CborLen)]
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
    pub fn from_socket_addr(socket_addr: SocketAddr) -> HostnamePort {
        let ip = match socket_addr.ip() {
            IpAddr::V4(ip) => ip.to_string(),
            IpAddr::V6(ip) => format!("[{ip}]"),
        };
        Self {
            hostname: ip,
            port: socket_addr.port(),
        }
    }

    /// Return a socket address from a hostname and port
    #[cfg(feature = "std")]
    pub fn to_socket_addr(&self) -> ockam_core::Result<SocketAddr> {
        crate::resolve_peer(self.to_string())
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

#[cfg(feature = "std")]
mod serde {
    use super::*;
    use ::serde::{Deserialize, Deserializer, Serialize, Serializer};
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
            HostnamePort::from_str(&s).map_err(::serde::de::Error::custom)
        }
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

        // otherwise check if brackets are present for an IP v6 address
        let ip_regex = if hostname_port.contains('[') {
            // we want to parse an IP v6 address as [hostname]:port where hostname does not contain [ or ]
            regex::Regex::new(r"(\[[^\[\]].*\]):(\d+)").unwrap()
        } else {
            regex::Regex::new(r"^([^:]*):(\d+)$").unwrap()
        };

        // Attempt to match the regular expression
        if let Some(captures) = ip_regex.captures(hostname_port) {
            if let (Some(hostname), Some(port)) = (captures.get(1), captures.get(2)) {
                if let Ok(port) = port.as_str().parse::<u16>() {
                    let mut hostname = hostname.as_str().to_string();
                    if hostname.is_empty() {
                        hostname = "127.0.0.1".to_string()
                    };
                    return Ok(HostnamePort { hostname, port });
                }
            }
        };

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
    use crate::resolve_peer;
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

        let socket_addr = resolve_peer("76.76.21.21:8080".to_string()).unwrap();
        let actual = HostnamePort::from_socket_addr(socket_addr);
        assert_eq!(actual, HostnamePort::from_str("76.76.21.21:8080").unwrap());

        let actual = HostnamePort::from_str("[2001:db8:85a3::8a2e:370:7334]:8080")?;
        assert_eq!(
            actual,
            HostnamePort::new("[2001:db8:85a3::8a2e:370:7334]", 8080)
        );

        let socket_addr = SocketAddr::from_str("[2001:db8:85a3::8a2e:370:7334]:8080").unwrap();
        let actual = HostnamePort::from_socket_addr(socket_addr);
        assert_eq!(
            actual,
            HostnamePort::new("[2001:db8:85a3::8a2e:370:7334]", 8080)
        );

        Ok(())
    }
}
