use std::fmt;
use std::{net::SocketAddr, str::FromStr};

use ockam_core::{Address, Result};
use ockam_node::Context;

use crate::{
    parse_socket_addr,
    router::{UdpRouter, UdpRouterHandle},
    UDP,
};

/// High level management interface for UDP transports
pub struct UdpTransport {
    router_handle: UdpRouterHandle,
}

impl UdpTransport {
    /// Create a new UDP transport and router for the current node
    pub async fn create(ctx: &Context) -> Result<UdpTransport> {
        let router_handle = UdpRouter::register(ctx).await?;
        Ok(Self { router_handle })
    }

    /// Start listening to incoming datagrams on an existing transport
    pub async fn listen<S: AsRef<str>>(&self, bind_addr: S) -> Result<()> {
        let bind_addr = parse_socket_addr(bind_addr)?;
        self.router_handle.bind(bind_addr).await
    }

    // TODO: connect method for manually connecting.
}

#[derive(Clone)]
pub(crate) struct UdpAddress {
    protocol: String,
    socket_addr: SocketAddr,
}

impl From<UdpAddress> for Address {
    fn from(other: UdpAddress) -> Self {
        format!("{}#{}", UDP, other.socket_addr).into()
    }
}

impl From<SocketAddr> for UdpAddress {
    fn from(socket_addr: SocketAddr) -> Self {
        Self {
            protocol: "udp".to_string(),
            socket_addr,
        }
    }
}

impl From<UdpAddress> for SocketAddr {
    fn from(other: UdpAddress) -> Self {
        other.socket_addr
    }
}

impl fmt::Display for UdpAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}://{}", &self.protocol, &self.socket_addr)
    }
}

impl From<&UdpAddress> for String {
    fn from(other: &UdpAddress) -> Self {
        other.to_string()
    }
}

impl FromStr for UdpAddress {
    type Err = ockam_core::Error;

    fn from_str(s: &str) -> Result<Self> {
        let socket_addr = parse_socket_addr(s)?;
        Ok(UdpAddress::from(socket_addr))
    }
}
