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

/// TCP address type constant
pub const TCP: u8 = 1;

impl TcpTransport {
    /// Create a TCP transport and establish an outgoing connection
    pub async fn create<S>(ctx: &Context, peer: S) -> Result<WorkerPair>
    where
        S: Into<String>,
    {
        let address: SocketAddr = peer.into().parse().map_err(|_| TcpError::InvalidAddress)?;
        init::start_connection(ctx, address).await
    }

    /// Create a TCP transport and listen for incoming connections
    pub async fn create_listener<'c, S>(
        ctx: &'c Context,
        bind_addr: S,
    ) -> Result<TcpRouterHandle<'c>>
    where
        S: Into<String>,
    {
        let address: SocketAddr = bind_addr
            .into()
            .parse()
            .map_err(|_| TcpError::InvalidAddress)?;
        TcpRouter::bind(ctx, address).await
    }
}
