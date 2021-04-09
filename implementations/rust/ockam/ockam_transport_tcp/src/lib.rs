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

#![deny(
    // missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unused_import_braces,
    unused_qualifications,
)]

#[macro_use]
extern crate tracing;

pub(crate) mod atomic;
mod error;
mod init;
mod listener;
mod receiver;
mod router;
mod sender;

pub use error::TcpError;
pub use init::WorkerPair;
pub use receiver::TcpRecvWorker;
pub use router::{TcpRouter, TcpRouterHandle};
pub use sender::TcpSendWorker;

use ockam::{Context, Result};
use std::net::SocketAddr;

/// An API layer object to control Ockam TCP transports
pub struct TcpTransport;

impl TcpTransport {
    /// Create a TCP transport and establish an outgoing connection
    pub async fn create<P>(ctx: &Context, peer: P) -> Result<WorkerPair>
    where
        P: Into<SocketAddr>,
    {
        init::start_connection(ctx, peer).await
    }

    /// Create a TCP transport and listen for incoming connections
    pub async fn create_listener<'c, P>(
        ctx: &'c Context,
        socket_addr: P,
    ) -> Result<TcpRouterHandle<'c>>
    where
        P: Into<SocketAddr>,
    {
        TcpRouter::bind(ctx, socket_addr).await
    }
}
