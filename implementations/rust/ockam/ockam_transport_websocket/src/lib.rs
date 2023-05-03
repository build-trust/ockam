//! This crate provides a WebSocket Transport for Ockam's Routing Protocol.
#![deny(unsafe_code)]
#![warn(
// missing_docs,
dead_code,
trivial_casts,
trivial_numeric_casts,
unused_import_braces,
unused_qualifications
)]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "std")]
extern crate core;
#[macro_use]
extern crate tracing;

use std::net::SocketAddr;

use ockam_core::{Result, TransportType};
use ockam_transport_core::TransportError;
pub use transport::*;

use crate::router::{WebSocketRouter, WebSocketRouterHandle};

mod error;
mod router;
mod transport;
mod workers;

/// WebSocket address type constant.
pub const WS: TransportType = TransportType::new(3);

pub(crate) const CLUSTER_NAME: &str = "_internals.transport.ws";

fn parse_socket_addr<S: AsRef<str>>(s: S) -> Result<SocketAddr> {
    Ok(s.as_ref()
        .parse()
        .map_err(|_| TransportError::InvalidAddress)?)
}
