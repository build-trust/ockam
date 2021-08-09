//! TCP Transport utilities for Ockam's routing framework
//!
//! The `ockam_node` (or `ockam_node_no_std`) crate sits at the core
//! of the Ockam routing framework, with transport specific
//! abstraction plugins.  This crate implements a TCP connection
//! plugin for this architecture.
//!
//! You can use Ockam's routing mechanism for cryptographic protocols,
//! key lifecycle, credential exchange, enrollment, etc, without having
//! to worry about the transport specifics.

#![deny(
    missing_docs,
    dead_code,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unused_import_braces,
    unused_qualifications
)]

pub(crate) mod atomic;

mod router;
mod workers;

pub(crate) use router::*;
pub(crate) use workers::*;

mod transport;

pub use transport::*;

use ockam_core::lib::net::SocketAddr;
use ockam_core::Result;
use ockam_transport::TransportError;

/// TCP address type constant
pub const TCP: u8 = 1;

fn parse_socket_addr(s: impl Into<String>) -> Result<SocketAddr> {
    Ok(s.into()
        .parse()
        .map_err(|_| TransportError::InvalidAddress)?)
}
