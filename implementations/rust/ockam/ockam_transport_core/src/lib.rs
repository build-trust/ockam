//! This crate provides the common code shared among the different Ockam's transport protocols.
//!
//! Each specific protocol is then supported in its own crate. For example, the TCP protocol is supported in the `ockam_transport_tcp` crate.
//!
//! Currently available transports include:
//!
//! * `ockam_transport_tcp` - TCP transport
//! * `ockam_transport_udp` - UDP transport
//! * `ockam_transport_ble` - Bluetooth Low Energy Transport
//! * `ockam_transport_websocket` - WebSocket Transport
//! * `ockam_transport_uds` - Unix Domain Socket Transport
//!

#![cfg_attr(not(feature = "std"), no_std)]

mod error;
mod hostname_port;
mod parse_socket;
mod scheme_hostname_port;
mod transport;

pub use error::TransportError;
pub use hostname_port::*;
pub use parse_socket::*;
pub use scheme_hostname_port::*;
pub use transport::*;
