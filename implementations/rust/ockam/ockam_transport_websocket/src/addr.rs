use ockam_core::lib::fmt;
use ockam_core::lib::net::SocketAddr;
use ockam_core::lib::str::FromStr;
use ockam_core::Error;

use crate::common::parse_socket_addr;

#[derive(Clone)]
pub struct WebSocketAddr {
    protocol: String,
    socket_addr: SocketAddr,
}

impl From<SocketAddr> for WebSocketAddr {
    fn from(socket_addr: SocketAddr) -> Self {
        Self {
            protocol: "ws".to_string(),
            socket_addr,
        }
    }
}

impl From<WebSocketAddr> for SocketAddr {
    fn from(other: WebSocketAddr) -> Self {
        other.socket_addr
    }
}

impl From<&WebSocketAddr> for String {
    fn from(other: &WebSocketAddr) -> Self {
        other.to_string()
    }
}

impl FromStr for WebSocketAddr {
    type Err = Error;

    fn from_str(s: &str) -> core::result::Result<Self, Self::Err> {
        let socket_addr = parse_socket_addr(s)?;
        Ok(WebSocketAddr::from(socket_addr))
    }
}

impl fmt::Display for WebSocketAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}://{}", &self.protocol, &self.socket_addr)
    }
}
