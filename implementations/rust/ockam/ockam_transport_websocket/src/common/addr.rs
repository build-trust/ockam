use ockam_core::lib::net::SocketAddr;
use ockam_core::Result;

use crate::common::TransportError;

pub fn parse_socket_addr<S: Into<String>>(s: S) -> Result<SocketAddr> {
    Ok(s.into()
        .parse()
        .map_err(|_| TransportError::InvalidAddress)?)
}
