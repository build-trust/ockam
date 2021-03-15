//! TCP Transport utilities for Ockam's routing framework
//!
//! The `ockam_node` (or `ockam_node_no_std`) crate sits at the core
//! of the Ockam routing framework, with transport specific
//! abstraction plugins.  This crate implements a TCP connection
//! plugin for this architecture.
//!
//! You can use Ockam's routing mechanism for cryptographic protocols,
//! key lifecycle, credetial exchange, enrollment, etc, without having
//! to worry about the transport specifics.

// FIXME: un-comment these when the code is ready
// #![deny(
//     // missing_docs,
//     trivial_casts,
//     trivial_numeric_casts,
//     unsafe_code,
//     unused_import_braces,
//     unused_qualifications,
// )]

mod error;
mod init;
mod receiver;
mod sender;

pub use error::TcpError;
pub use init::start_tcp_worker;
pub use receiver::TcpRecvWorker;
pub use sender::TcpSendWorker;
