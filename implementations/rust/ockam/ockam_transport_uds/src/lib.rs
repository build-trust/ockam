#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;
// TODO: Are there any known uses cases for no_std with UDS
#[cfg(feature = "std")]
extern crate core;

mod router;
mod transport;
mod workers;
use tokio::net::unix::SocketAddr as TokioSocketAddr;
use tracing::error;
pub use transport::*;

use std::os::unix::net::SocketAddr;

use ockam_core::{Address, Result, TransportType};
use ockam_transport_core::TransportError;

// TODO: Should have documentation or enum for this
pub const UDS: TransportType = TransportType::new(5);

pub const CLUSTER_NAME: &str = "_internals.transport.uds";

fn parse_socket_addr<S: AsRef<str>>(s: S) -> Result<SocketAddr> {
    Ok(SocketAddr::from_pathname(s.as_ref()).map_err(|_| TransportError::InvalidAddress)?)
}

fn std_socket_addr_from_tokio(sock_addr: &TokioSocketAddr) -> Result<SocketAddr> {
    let path = match sock_addr.as_pathname() {
        Some(p) => p,
        None => {
            error!("Error retrieving path from tokio Socket Addr");
            return Err(TransportError::InvalidAddress.into());
        }
    };

    let sock = match SocketAddr::from_pathname(path) {
        Ok(s) => s,
        Err(e) => {
            error!("Error parsing std SocketAddr from Tokio SocketAddr: {}", e);
            return Err(TransportError::InvalidAddress.into());
        }
    };

    Ok(sock)
}

fn address_from_socket_addr(sock_addr: &SocketAddr) -> Result<Address> {
    let path = match sock_addr.as_pathname() {
        Some(p) => p,
        None => {
            return Err(TransportError::InvalidAddress.into());
        }
    };

    let path_str = match path.to_str() {
        Some(s) => s,
        None => {
            return Err(TransportError::InvalidAddress.into());
        }
    };

    let address: Address = format!("{UDS}#{path_str}").into();

    Ok(address)
}

#[test]
fn test_parse_socket_address() {
    let result = parse_socket_addr("/tmp/sock");
    assert!(result.is_ok());
}
