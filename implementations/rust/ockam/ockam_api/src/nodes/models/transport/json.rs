#![allow(clippy::unconditional_recursion)]
use std::fmt::Display;

use crate::cli_state::CliStateError;
use crate::config::lookup::InternetAddress;
use crate::nodes::models::transport::{TransportMode, TransportType};
use crate::Result;
use ockam_multiaddr::proto::{DnsAddr, Ip4, Ip6, Tcp};
use ockam_multiaddr::MultiAddr;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash)]
pub struct CreateTransportJson {
    pub tt: TransportType,
    /// The mode the transport should operate in
    pub tm: TransportMode,
    /// The address payload for the transport
    pub addr: InternetAddress,
}

impl CreateTransportJson {
    pub fn new(tt: TransportType, tm: TransportMode, addr: &str) -> Result<Self> {
        Ok(Self {
            tt,
            tm,
            addr: InternetAddress::new(addr).ok_or_else(|| {
                CliStateError::InvalidOperation("Invalid address '{addr}'".to_string())
            })?,
        })
    }

    pub fn maddr(&self) -> Result<MultiAddr> {
        let mut m = MultiAddr::default();
        let addr = &self.addr;
        match addr {
            InternetAddress::Dns(dns, _) => m.push_back(DnsAddr::new(dns))?,
            InternetAddress::V4(v4) => m.push_back(Ip4(*v4.ip()))?,
            InternetAddress::V6(v6) => m.push_back(Ip6(*v6.ip()))?,
        }
        m.push_back(Tcp(addr.port()))?;
        Ok(m)
    }
}

impl Display for CreateTransportJson {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}/{}", self.tt, self.tm, self.addr)
    }
}
