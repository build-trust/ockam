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
    dead_code,
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

use ockam::{Address, Context, Result};
use std::net::SocketAddr;

/// An API layer object to control Ockam TCP transports
pub struct TcpTransport<'ctx> {
    ctx: &'ctx Context,
    router: TcpRouterHandle<'ctx>,
}

/// TCP address type constant
pub const TCP: u8 = 1;

fn parse_socket_addr<S: Into<String>>(s: S) -> Result<SocketAddr> {
    Ok(s.into().parse().map_err(|_| TcpError::InvalidAddress)?)
}

impl<'ctx> TcpTransport<'ctx> {
    /// Create a TCP transport and establish an outgoing connection
    pub async fn create<S: Into<String>>(
        ctx: &'ctx Context,
        peer: S,
    ) -> Result<TcpTransport<'ctx>> {
        let addr = Address::random(0);
        let peer = parse_socket_addr(peer)?;

        let router = TcpRouter::register(ctx, addr.clone()).await?;
        init::start_connection(ctx, &router, peer).await?;

        Ok(Self { ctx, router })
    }

    /// Establish an outgoing TCP connection on an existing transport
    pub async fn connect<S: Into<String>>(&self, peer: S) -> Result<()> {
        let peer = parse_socket_addr(peer)?;
        init::start_connection(&self.ctx, &self.router, peer).await?;
        Ok(())
    }

    /// Create a TCP transport and listen for incoming connections
    pub async fn create_listener<S: Into<String>>(
        ctx: &'ctx Context,
        bind_addr: S,
    ) -> Result<TcpTransport<'ctx>> {
        let addr = Address::random(0);
        let bind_addr = parse_socket_addr(bind_addr)?;
        let router = TcpRouter::bind(ctx, addr, bind_addr).await?;
        Ok(Self { ctx, router })
    }
}
