//! This crate provides a UDP Transport for Ockam's Routing Protocol.
//!
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

mod error;
mod messages;
mod options;
mod puncture;
mod transport;
mod workers;

pub use error::*;
pub use options::UdpBindOptions;
pub use puncture::*;
pub use transport::{UdpBind, UdpBindArguments, UdpTransport, UdpTransportExtension};

pub(crate) const CLUSTER_NAME: &str = "_internals.transport.udp";

/// Transport type for UDP addresses
pub const UDP: ockam_core::TransportType = ockam_core::TransportType::new(2);

/// 16 MB
pub const MAX_MESSAGE_SIZE: usize = 16 * 1024 * 1024;
