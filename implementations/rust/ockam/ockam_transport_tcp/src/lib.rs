//! This crate provides a TCP Transport for Ockam's Routing Protocol.
//!
//! This crate requires the rust standard library `"std"`
#![deny(unsafe_code)]
#![warn(
    missing_docs,
    dead_code,
    trivial_casts,
    trivial_numeric_casts,
    unused_import_braces,
    unused_qualifications
)]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate core;

#[cfg(feature = "alloc")]
extern crate alloc;

mod options;
mod portal;
mod registry;
mod transport;

use ockam_core::TransportType;
pub use options::{TcpConnectionOptions, TcpListenerOptions};
pub use portal::{PortalInternalMessage, PortalMessage, MAX_PAYLOAD_SIZE};
pub use registry::*;
pub use transport::*;

mod workers;
pub(crate) use workers::*;

pub(crate) const CLUSTER_NAME: &str = "_internals.transport.tcp";

/// Transport type for TCP addresses
pub const TCP: TransportType = TransportType::new(1);
