//! This crate provides a TCP Transport for Ockam's Routing Protocol.
//!
//! The Routing Protocol decouples Ockam's suite of cryptographic protocols,
//! like secure channels, key lifecycle, credential exchange, enrollment etc. from
//! the underlying transport protocols. This allows applications to establish
//! end-to-end trust between entities, independently from the underlying transport.

pub mod connection;
pub mod error;
pub mod listener;
pub mod serializer;
pub mod transport_traits;

pub use connection::*;
pub use error::*;
pub use listener::*;
pub use serializer::*;
pub use transport_traits::*;
