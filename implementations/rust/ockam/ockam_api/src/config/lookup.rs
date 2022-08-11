use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fmt,
    net::{SocketAddr, SocketAddrV4, SocketAddrV6},
    str::FromStr,
};

/// A generic lookup mechanism for configuration values
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConfigLookup {
    #[serde(flatten)]
    pub map: BTreeMap<String, LookupValue>,
}

impl Default for ConfigLookup {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigLookup {
    pub fn new() -> Self {
        Self {
            map: Default::default(),
        }
    }

    /// Store a node identifier and address lookup
    pub fn set_node(&mut self, node: &str, address: InternetAddress) {
        self.map
            .insert(format!("/node/{}", node), LookupValue::Address(address));
    }

    pub fn get_node(&self, node: &str) -> Option<&InternetAddress> {
        self.map
            .get(&format!("/node/{}", node))
            .and_then(|value| match value {
                LookupValue::Address(addr) => Some(addr),
                #[allow(unreachable_patterns)]
                _ => None,
            })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LookupValue {
    Address(InternetAddress),
}

/// An internet address abstraction (v6/v4/dns)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum InternetAddress {
    /// DNSaddr and port
    Dns(String, u16),
    /// An IPv4 socket address
    V4(SocketAddrV4),
    /// An IPv6 socket address
    V6(SocketAddrV6),
}

impl fmt::Display for InternetAddress {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(
            match self {
                Self::Dns(addr, port) => format!("{}:{}", addr, port),
                Self::V4(v4) => format!("{}", v4),
                Self::V6(v6) => format!("{}", v6),
            }
            .as_str(),
        )
    }
}

impl InternetAddress {
    pub fn new(addr: &str) -> Option<Self> {
        // We try to parse a SocketAddress first, and if this fails
        // then assume it's a DNS address
        match SocketAddr::from_str(addr) {
            Ok(addr) => match addr {
                SocketAddr::V4(v4) => Some(Self::V4(v4)),
                SocketAddr::V6(v6) => Some(Self::V6(v6)),
            },
            Err(_) => {
                let addr_parts: Vec<&str> = addr.split(':').collect();
                if addr_parts.len() != 2 {
                    return None;
                }

                Some(Self::Dns(
                    addr_parts[0].to_string(),
                    addr_parts[1].parse().ok()?,
                ))
            }
        }
    }

    pub fn from_dns(s: String, port: u16) -> Self {
        Self::Dns(s, port)
    }

    /// Get the port for this address
    pub fn port(&self) -> u16 {
        match self {
            Self::Dns(_, port) => *port,
            Self::V4(v4) => v4.port(),
            Self::V6(v6) => v6.port(),
        }
    }
}

impl From<SocketAddr> for InternetAddress {
    fn from(sa: SocketAddr) -> Self {
        match sa {
            SocketAddr::V4(v4) => Self::V4(v4),
            SocketAddr::V6(v6) => Self::V6(v6),
        }
    }
}
