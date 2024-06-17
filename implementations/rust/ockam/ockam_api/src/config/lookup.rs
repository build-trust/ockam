use minicbor::{CborLen, Decode, Encode};
use ockam_core::compat::collections::VecDeque;
use ockam_multiaddr::proto::{DnsAddr, Ip4, Ip6, Tcp};
use ockam_multiaddr::MultiAddr;
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    net::{SocketAddr, SocketAddrV4, SocketAddrV6},
    str::FromStr,
};

#[derive(Debug, Default)]
pub struct LookupMeta {
    /// Append any project name that is encountered during look-up
    pub project: VecDeque<Name>,
}

pub type Name = String;

/// A generic lookup
/// An internet address abstraction (v6/v4/dns)
#[derive(Clone, Debug, Serialize, Deserialize, Encode, Decode, CborLen, PartialEq, Eq, Hash)]
#[rustfmt::skip]
#[serde(untagged)]
pub enum InternetAddress {
    /// DNSaddr and port
    #[n(0)] Dns(#[n(0)] String, #[n(1)] u16),
    /// An IPv4 socket address
    #[n(1)] V4(#[n(0)] SocketAddrV4),
    /// An IPv6 socket address
    #[n(2)] V6(#[n(0)] SocketAddrV6),
}

impl InternetAddress {
    pub fn multi_addr(&self) -> ockam_core::Result<MultiAddr> {
        let mut m = MultiAddr::default();
        match self {
            InternetAddress::Dns(dns, _) => m.push_back(DnsAddr::new(dns))?,
            InternetAddress::V4(v4) => m.push_back(Ip4(*v4.ip()))?,
            InternetAddress::V6(v6) => m.push_back(Ip6(*v6.ip()))?,
        }
        m.push_back(Tcp(self.port()))?;
        Ok(m)
    }
}

impl Default for InternetAddress {
    fn default() -> Self {
        InternetAddress::Dns("localhost".to_string(), 6252)
    }
}

impl fmt::Display for InternetAddress {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(
            match self {
                Self::Dns(addr, port) => format!("{addr}:{port}"),
                Self::V4(v4) => format!("{v4}"),
                Self::V6(v6) => format!("{v6}"),
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
