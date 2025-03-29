use crate::{HostnamePort, StaticHostnamePort};
use core::fmt::{Display, Formatter};
use core::str::FromStr;
use ockam_core::compat::{format, string::String, vec::Vec};
use ockam_core::errcode::{Kind, Origin};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SchemeHostnamePort {
    scheme: ValidScheme,
    hostname_port: HostnamePort,
}

impl SchemeHostnamePort {
    pub fn new(
        scheme: impl Into<String>,
        hostname: impl Into<String>,
        port: u16,
    ) -> ockam_core::Result<Self> {
        let scheme = scheme.into();
        Ok(SchemeHostnamePort {
            scheme: ValidScheme::from_str(&scheme)?,
            hostname_port: HostnamePort::new(hostname, port)?,
        })
    }

    pub fn hostname_port(&self) -> &HostnamePort {
        &self.hostname_port
    }

    pub fn hostname(&self) -> String {
        self.hostname_port.hostname()
    }

    pub fn port(&self) -> u16 {
        self.hostname_port.port()
    }

    pub fn is_tls(&self) -> bool {
        self.scheme == ValidScheme::Tls
    }

    pub fn is_udp(&self) -> bool {
        self.scheme == ValidScheme::Udp
    }
}

impl From<SchemeHostnamePort> for HostnamePort {
    fn from(scheme_hostname_port: SchemeHostnamePort) -> Self {
        scheme_hostname_port.hostname_port
    }
}

impl From<&SchemeHostnamePort> for HostnamePort {
    fn from(scheme_hostname_port: &SchemeHostnamePort) -> Self {
        scheme_hostname_port.hostname_port.clone()
    }
}

impl TryFrom<StaticHostnamePort> for SchemeHostnamePort {
    type Error = ockam_core::Error;

    fn try_from(value: StaticHostnamePort) -> Result<Self, Self::Error> {
        Ok(SchemeHostnamePort {
            scheme: ValidScheme::Tcp,
            hostname_port: value.try_into()?,
        })
    }
}

impl Display for SchemeHostnamePort {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.write_str(&format!("{}://{}", self.scheme, self.hostname_port))
    }
}

impl FromStr for SchemeHostnamePort {
    type Err = <HostnamePort as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // The input string can be either "scheme://hostname:port" or "hostname:port"
        // If the scheme is missing, we assume it's "tcp"
        let mut parts = s.split("://");
        let (scheme, hostname_port) = match (parts.next(), parts.next()) {
            (Some(s), Some(hp)) => (s, hp),
            (Some(hp), None) => ("tcp", hp),
            _ => {
                return Err(ockam_core::Error::new(
                    Origin::Application,
                    Kind::Serialization,
                    "invalid input string for SchemeHostnamePort",
                ));
            }
        };
        Ok(SchemeHostnamePort {
            scheme: ValidScheme::from_str(scheme)?,
            hostname_port: HostnamePort::from_str(hostname_port)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ValidScheme {
    Tcp,
    Udp,
    Tls,
}

impl ValidScheme {
    pub fn values() -> &'static [Self] {
        &[ValidScheme::Tcp, ValidScheme::Udp, ValidScheme::Tls]
    }
}

impl AsRef<str> for ValidScheme {
    fn as_ref(&self) -> &str {
        match self {
            ValidScheme::Tcp => "tcp",
            ValidScheme::Udp => "udp",
            ValidScheme::Tls => "tls",
        }
    }
}

impl FromStr for ValidScheme {
    type Err = ockam_core::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "tcp" => Ok(ValidScheme::Tcp),
            "udp" => Ok(ValidScheme::Udp),
            "tls" => Ok(ValidScheme::Tls),
            _ => Err(ockam_core::Error::new(
                Origin::Application,
                Kind::Serialization,
                format!(
                    "invalid scheme {s}. Supported schemes are {}",
                    ValidScheme::values()
                        .iter()
                        .map(|s| s.as_ref())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            )),
        }
    }
}

impl Display for ValidScheme {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_scheme() {
        let scheme = ValidScheme::from_str("tcp").unwrap();
        assert_eq!(scheme, ValidScheme::Tcp);
        assert_eq!(scheme.as_ref(), "tcp");

        let scheme = ValidScheme::from_str("udp").unwrap();
        assert_eq!(scheme, ValidScheme::Udp);
        assert_eq!(scheme.as_ref(), "udp");

        let scheme = ValidScheme::from_str("tls").unwrap();
        assert_eq!(scheme, ValidScheme::Tls);
        assert_eq!(scheme.as_ref(), "tls");
    }

    #[test]
    fn test_scheme_hostname_port() {
        let valid_cases = [
            ("tcp://localhost:22", ValidScheme::Tcp, "localhost:22"),
            ("udp://my.domain:1234", ValidScheme::Udp, "my.domain:1234"),
            ("tls://127.0.0.1:45678", ValidScheme::Tls, "127.0.0.1:45678"),
            ("localhost:1234", ValidScheme::Tcp, "localhost:1234"),
            ("6543", ValidScheme::Tcp, "127.0.0.1:6543"),
        ];
        for (input, expected_scheme, expected_hostname_port) in valid_cases.iter() {
            let scheme_hostname_port = SchemeHostnamePort::from_str(input).unwrap();
            assert_eq!(&scheme_hostname_port.scheme, expected_scheme);
            assert_eq!(
                scheme_hostname_port.hostname_port.to_string(),
                expected_hostname_port.to_string()
            );
        }

        let invalid_cases = ["", "tcp://", "tcp://localhost"];
        for input in invalid_cases.iter() {
            assert!(SchemeHostnamePort::from_str(input).is_err());
        }
    }
}
