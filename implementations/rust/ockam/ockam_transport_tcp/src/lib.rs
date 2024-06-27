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
mod protocol_version;
mod registry;
mod transport;
mod transport_message;
mod workers;

pub(crate) use workers::*;

pub use options::{TcpConnectionOptions, TcpListenerOptions};
pub use portal::{
    new_certificate_provider_cache, Direction, PortalInletInterceptor, PortalInterceptor,
    PortalInterceptorFactory, PortalInterceptorWorker, PortalInternalMessage, PortalMessage,
    PortalOutletInterceptor, TlsCertificate, TlsCertificateProvider, MAX_PAYLOAD_SIZE,
};
pub use protocol_version::*;
pub use registry::*;
pub use transport::*;

pub(crate) const CLUSTER_NAME: &str = "_internals.transport.tcp";

/// Transport type for TCP addresses
pub const TCP: ockam_core::TransportType = ockam_core::TransportType::new(1);

/// 16 MB
pub const MAX_MESSAGE_SIZE: usize = 16 * 1024 * 1024;
