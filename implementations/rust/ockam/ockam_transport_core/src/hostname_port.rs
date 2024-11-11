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
#[cfg(feature = "std")]
use url::Url;

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

impl TryFrom<StaticHostnamePort> for HostnamePort {
    type Error = ockam_core::Error;

    fn try_from(value: StaticHostnamePort) -> ockam_core::Result<Self> {
        HostnamePort::new(value.hostname, value.port)
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
    pub fn new(hostname: impl Into<String>, port: u16) -> ockam_core::Result<HostnamePort> {
        let _self = Self {
            hostname: hostname.into(),
            port,
        };
        Self::validate(&_self.to_string())
    }

    /// Return the hostname
    pub fn hostname(&self) -> String {
        self.hostname.clone()
    }

    /// Return the port
    pub fn port(&self) -> u16 {
        self.port
    }

    #[cfg(feature = "std")]
    pub fn into_url(self, scheme: &str) -> ockam_core::Result<Url> {
        Url::parse(&format!("{}://{}:{}", scheme, self.hostname, self.port))
            .map_err(|_| ockam_core::Error::new(Origin::Api, Kind::Serialization, "invalid url"))
    }

    fn validate(hostname_port: &str) -> ockam_core::Result<Self> {
        // Check if the input is an IP address
        if let Ok(socket) = parse_socket_addr(hostname_port) {
            return Ok(HostnamePort::from(socket));
        }

        // Split the input into hostname and port
        let (hostname, port_str) = match hostname_port.split_once(':') {
            None => {
                return Err(ockam_core::Error::new(
                    Origin::Api,
                    Kind::Serialization,
                    "Invalid format. Expected 'hostname:port'".to_string(),
                ))
            }
            Some((hostname, port_str)) => (hostname, port_str),
        };

        // Validate port
        let port = port_str.parse::<u16>().map_err(|_| {
            ockam_core::Error::new(
                Origin::Api,
                Kind::Serialization,
                format!("Invalid port number {port_str}"),
            )
        })?;

        // Ensure the hostname is a valid ASCII string
        if !hostname.is_ascii() {
            return Err(ockam_core::Error::new(
                Origin::Api,
                Kind::Serialization,
                format!("Hostname must be ascii: {hostname_port}"),
            ));
        }

        // Validate hostname
        if hostname.is_empty() {
            return Err(ockam_core::Error::new(
                Origin::Api,
                Kind::Serialization,
                format!("Hostname cannot be empty {hostname}"),
            ));
        }

        // The total length of the hostname should not exceed 253 characters
        if hostname.len() > 253 {
            return Err(ockam_core::Error::new(
                Origin::Api,
                Kind::Serialization,
                format!("Hostname too long {hostname}"),
            ));
        }

        // Hostname should not start or end with a hyphen or dot
        if hostname.starts_with('-')
            || hostname.ends_with('-')
            || hostname.starts_with('.')
            || hostname.ends_with('.')
        {
            return Err(ockam_core::Error::new(
                Origin::Api,
                Kind::Serialization,
                format!("Hostname cannot start or end with a hyphen or dot {hostname}"),
            ));
        }

        // Check segments of the hostname
        for segment in hostname.split('.') {
            // Segment can't be empty (i.e. two dots in a row)
            if segment.is_empty() {
                return Err(ockam_core::Error::new(
                    Origin::Api,
                    Kind::Serialization,
                    format!("Hostname segment cannot be empty {hostname}"),
                ));
            }

            // Hostname segments (between dots) should be between 1 and 63 characters long
            if segment.len() > 63 {
                return Err(ockam_core::Error::new(
                    Origin::Api,
                    Kind::Serialization,
                    format!("Hostname segment too long {hostname}"),
                ));
            }
            // Hostname can contain alphanumeric characters, hyphens (-), dots (.), and underscores (_)
            if !segment
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
            {
                return Err(ockam_core::Error::new(
                    Origin::Api,
                    Kind::Serialization,
                    format!("Hostname contains invalid characters {hostname}"),
                ));
            }
        }

        Ok(Self {
            hostname: hostname.to_string(),
            port,
        })
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
            return HostnamePort::new("127.0.0.1", port);
        }

        if let Some(port_str) = hostname_port.strip_prefix(':') {
            if let Ok(port) = port_str.parse::<u16>() {
                return HostnamePort::new("127.0.0.1", port);
            }
        }

        if let Ok(socket) = parse_socket_addr(hostname_port) {
            return Ok(HostnamePort::from(socket));
        }

        // We now know it's not an ip, let's validate if it can be a valid hostname
        Self::validate(hostname_port)
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
    fn hostname_port_valid_inputs() -> ockam_core::Result<()> {
        let valid_cases = vec![
            ("localhost:80", HostnamePort::new("localhost", 80)?),
            ("33domain:80", HostnamePort::new("33domain", 80)?),
            ("127.0.0.1:80", HostnamePort::new("127.0.0.1", 80)?),
            ("xn--74h.com:80", HostnamePort::new("xn--74h.com", 80)?),
            (
                "sub.xn_74h.com:80",
                HostnamePort::new("sub.xn_74h.com", 80)?,
            ),
            (":80", HostnamePort::new("127.0.0.1", 80)?),
            ("80", HostnamePort::new("127.0.0.1", 80)?),
            (
                "[2001:db8:85a3::8a2e:370:7334]:8080",
                HostnamePort::new("[2001:db8:85a3::8a2e:370:7334]", 8080)?,
            ),
            ("[::1]:8080", HostnamePort::new("[::1]", 8080)?),
            (
                "[2001:db8:85a3::8a2e:370:7334]:8080",
                HostnamePort::new("[2001:db8:85a3::8a2e:370:7334]", 8080)?,
            ),
        ];
        for (input, expected) in valid_cases {
            let actual = HostnamePort::from_str(input).ok().unwrap();
            assert_eq!(actual, expected);
        }

        let socket_address_cases = vec![
            (
                SocketAddr::from_str("127.0.0.1:8080").unwrap(),
                HostnamePort::new("127.0.0.1", 8080)?,
            ),
            (
                SocketAddr::from_str("[2001:db8:85a3::8a2e:370:7334]:8080").unwrap(),
                HostnamePort::new("[2001:db8:85a3::8a2e:370:7334]", 8080)?,
            ),
            (
                SocketAddr::from_str("[::1]:8080").unwrap(),
                HostnamePort::new("[::1]", 8080)?,
            ),
        ];
        for (input, expected) in socket_address_cases {
            let actual = HostnamePort::from(input);
            assert_eq!(actual, expected);
        }

        Ok(())
    }

    #[test]
    fn hostname_port_invalid_inputs() {
        let cases = [
            "invalid",
            "localhost:80:80",
            "192,166,0.1:9999",
            "-hostname-with-leading-hyphen:80",
            "hostname-with-trailing-hyphen-:80",
            ".hostname-with-leading-dot:80",
            "hostname-with-trailing-dot.:80",
            "hostname..with..multiple..dots:80",
            "hostname_with_invalid_characters!@#:80",
            "hostname_with_ space:80",
            "hostname_with_backslash\\:80",
            "hostname_with_slash/:80",
            "hostname_with_colon::80",
            "hostname_with_semicolon;:80",
            "hostname_with_quote\":80",
            "hostname_with_single_quote':80",
            "hostname_with_question_mark?:80",
            "hostname_with_asterisk*:80",
            "hostname_with_ampersand&:80",
            "hostname_with_percent%:80",
            "hostname_with_dollar$:80",
            "hostname_with_hash#:80",
            "hostname_with_at@:80",
            "hostname_with_exclamation!:80",
            "hostname_with_tilde~:80",
            "hostname_with_caret^:80",
            "hostname_with_open_bracket[:80",
            "hostname_with_close_bracket]:80",
            "hostname_with_open_brace{:80",
            "hostname_with_close_brace}:80",
            "hostname_with_open_parenthesis(:80",
            "hostname_with_close_parenthesis):80",
            "hostname_with_plus+:80",
            "hostname_with_equal=:80",
            "hostname_with_comma,:80",
            "hostname_with_less_than<:80",
            "hostname_with_greater_than>:80",
        ];
        for case in cases.iter() {
            if HostnamePort::from_str(case).is_ok() {
                panic!("HostnamePort should fail for '{case}'");
            }
        }
    }
}
