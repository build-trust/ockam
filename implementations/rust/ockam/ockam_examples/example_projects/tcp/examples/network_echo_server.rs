//! This example is part of `network_echo`
//!
//! You need to start this binary first, before letting the
//! `network_echo_client` connect to it.

#[macro_use]
extern crate tracing;

use ockam::{Context, Result, Routed, Worker};
use ockam_transport_tcp::TcpTransport;

struct Responder;

#[ockam::worker]
impl Worker for Responder {
    type Context = Context;
    type Message = String;

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<String>) -> Result<()> {
        info!("Responder: {}", msg);
        ctx.send(msg.return_route(), msg.body()).await?;
        Ok(())
    }
}

fn get_bind_addr() -> String {
    std::env::args()
        .skip(1)
        .take(1)
        .next()
        .unwrap_or(format!("127.0.0.1:10222"))
}

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Get either the default socket address, or a user-input
    let bind_addr = get_bind_addr();
    debug!("Binding to: {}", bind_addr);
    let tcp = TcpTransport::create(&ctx).await?;
    tcp.listen(bind_addr).await?;

    // Create the responder worker
    ctx.start_worker("echo_service", Responder).await?;

    // The server never shuts down
    Ok(())
}
