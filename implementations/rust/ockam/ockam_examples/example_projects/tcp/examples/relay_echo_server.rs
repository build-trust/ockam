#[macro_use]
extern crate tracing;

use ockam::{async_worker, Context, Result, Route, Routed, Worker};
use ockam_transport_tcp::TcpTransport;
use std::net::SocketAddr;

fn get_peer_addr() -> SocketAddr {
    std::env::args()
        .skip(1)
        .take(1)
        .next()
        // This value can be used when running the ockam-hub locally
        .unwrap_or(format!("127.0.0.1:4000"))
        .parse()
        .ok()
        .unwrap_or_else(|| {
            error!("Failed to parse socket address!");
            eprintln!("Usage: network_echo_client <ip>:<port>");
            std::process::exit(1);
        })
}

/// A worker who isn't always available.  It registers itself with the
/// hub forwarding service to never miss a message.
struct ProxiedWorker {
    /// The hub's address
    peer: SocketAddr,
}

#[async_worker]
impl Worker for ProxiedWorker {
    type Context = Context;
    type Message = String;

    async fn initialize(&mut self, ctx: &mut Context) -> Result<()> {
        // Register this service with the hub's forwarding service
        ctx.send(
            Route::new()
                .append(format!("1#{}", self.peer))
                .append("forwarding_service"),
            String::from("register"),
        )
        .await
    }

    async fn handle_message(&mut self, _: &mut Context, msg: Routed<Self::Message>) -> Result<()> {
        // This condition is true when we receive the forwarding route
        // from registration - print this route for the user to copy
        if &msg.as_str() == &"register" {
            info!("You can reach me via this route: {}", msg.return_route());
        }
        // This condition is true when we receive a message that is
        // being forwarded
        else {
            println!("Forwarded message: {}", msg.body());
        }

        Ok(())
    }
}

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Get our peer address
    let peer = get_peer_addr();

    // Create and register a connection worker pair
    TcpTransport::create(&ctx, peer.clone()).await?;

    // Start the worker we want to reach via proxy
    ctx.start_worker("worker", ProxiedWorker { peer }).await?;

    Ok(())
}
