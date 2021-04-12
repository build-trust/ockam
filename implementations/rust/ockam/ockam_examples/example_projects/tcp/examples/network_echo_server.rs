//! This example is part of `network_echo`
//!
//! You need to start this binary first, before letting the
//! `network_echo_client` connect to it.

#[macro_use]
extern crate tracing;

use ockam::{async_worker, Context, Result, Routed, Worker};
use ockam_transport_tcp::TcpTransport;
use std::net::SocketAddr;

struct Responder;

#[async_worker]
impl Worker for Responder {
    type Context = Context;
    type Message = String;

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<String>) -> Result<()> {
        info!("Responder: {}", msg);
        ctx.send(msg.reply(), msg.take()).await?;
        Ok(())
    }
}

fn get_bind_addr() -> SocketAddr {
    std::env::args()
        .skip(1)
        .take(1)
        .next()
        .unwrap_or(format!("127.0.0.1:10222"))
        .parse()
        .ok()
        .unwrap_or_else(|| {
            error!("Failed to parse socket address!");
            eprintln!("Usage: network_echo_server <ip>:<port>");
            std::process::exit(1);
        })
}

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Get either the default socket address, or a user-input
    let bind_addr = get_bind_addr();
    debug!("Binding to: {}", bind_addr);
    TcpTransport::create_listener(&ctx, bind_addr).await?;

    // Create the responder worker
    ctx.start_worker("echo_service", Responder).await?;

    // The server never shuts down
    Ok(())
}
