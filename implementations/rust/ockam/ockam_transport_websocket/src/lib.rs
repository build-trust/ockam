//! This crate provides a WebSocket Transport for Ockam's Routing Protocol.
//!
//! This crate requires the rust standard library `"std"`.
//!
//! We need to define the behavior of the worker that will be processing incoming messages.
//!
//! ```rust,no_run
//! use ockam_core::{Worker, Result, Routed, async_trait};
//! use ockam_node::Context;
//!
//! struct MyWorker;
//!
//! #[async_trait]
//! impl Worker for MyWorker {
//!     type Context = Context;
//!     type Message = String;
//!
//!     async fn handle_message(&mut self, _ctx: &mut Context, _msg: Routed<String>) -> Result<()> {
//!         // ...
//!         Ok(())
//!     }
//! }
//!
//! // Now we can write the main function that will run the previous worker. In this case, our worker will be listening for new connections on port 8000 until the process is manually killed.
//!
//! use ockam_transport_websocket::WebSocketTransport;
//! use ockam_node::NodeBuilder;
//! use ockam_macros::node;
//!
//! #[ockam_macros::node(crate = "ockam_node")]
//! async fn main(mut ctx: Context) -> Result<()> {//!
//!     let ws = WebSocketTransport::create(&ctx)?;
//!     ws.listen("localhost:8000").await?; // Listen on port 8000
//!
//!     // Start a worker, of type MyWorker, at address "my_worker"
//!     ctx.start_worker("my_worker", MyWorker)?;
//!
//!     // Run worker indefinitely in the background
//!     Ok(())
//! }
//! ```
//!
//! Finally, we can write another node that connects to the node that is hosting the `MyWorker` worker, and we are ready to send and receive messages between them.
//!
//! ```rust,no_run
//! use ockam_transport_websocket::{WebSocketTransport, WS};
//! use ockam_core::{route, Result};
//! use ockam_node::Context;
//! use ockam_macros::node;
//!
//! #[ockam_macros::node(crate = "ockam_node")]
//! async fn main(mut ctx: Context) -> Result<()> {
//!     use ockam_node::MessageReceiveOptions;
//!     let ws = WebSocketTransport::create(&ctx)?;
//!
//!     // Define the route to the server's worker.
//!     let r = route![(WS, "localhost:8000"), "my_worker"];
//!
//!     // Now you can send messages to the worker.
//!     ctx.send(r, "Hello Ockam!".to_string()).await?;
//!
//!     // Or receive messages from the server.
//!     let reply = ctx.receive::<String>().await?;
//!
//!     // Stop all workers, stop the node, cleanup and return.
//!     ctx.shutdown_node().await
//! }
//! ```
//!
#![deny(unsafe_code)]
#![warn(
// missing_docs,
dead_code,
trivial_casts,
trivial_numeric_casts,
unused_import_braces,
unused_qualifications
)]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "std")]
extern crate core;
#[macro_use]
extern crate tracing;

use std::net::SocketAddr;

use ockam_core::{Result, TransportType};
use ockam_transport_core::TransportError;
pub use transport::*;

use crate::router::{WebSocketRouter, WebSocketRouterHandle};

mod error;
mod router;
mod transport;
mod workers;

/// WebSocket address type constant.
pub const WS: TransportType = TransportType::new(3);

fn parse_socket_addr<S: AsRef<str>>(s: S) -> Result<SocketAddr> {
    Ok(s.as_ref()
        .parse()
        .map_err(|_| TransportError::InvalidAddress(s.as_ref().to_string()))?)
}
