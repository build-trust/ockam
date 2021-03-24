#[macro_use]
extern crate tracing;

use ockam::{async_worker, Context, Result, Route, Routed, Worker};
use ockam_transport_tcp::{self as tcp, TcpRouter};
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
        ctx.send_message(
            Route::new()
                .append(format!("1#{}", self.peer))
                .append("forwarding_service"),
            String::from("register"),
        )
        .await
    }

    async fn handle_message(
        &mut self,
        ctx: &mut Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        // This condition is true when we receive the forwarding route
        // from our registration - forward this to the app.
        if &msg.as_str() == &"register" {
            println!("Receiving message: {}", msg);
            ctx.send_message("app", msg.reply()).await?;
        }
        // This condition is true when we receive a message that is
        // being forwarded
        else {
            println!("Forwarded message: {}", msg.take());
        }

        Ok(())
    }
}

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Get our peer address
    let peer = get_peer_addr();

    // Create and register a TcpRouter
    let rh = TcpRouter::register(&ctx).await?;

    // Create and register a connection worker pair
    let w_pair = tcp::start_tcp_worker(&ctx, peer.clone()).await?;
    rh.register(&w_pair).await?;

    ctx.start_worker("not.always.there", ProxiedWorker { peer })
        .await?;

    // Receive the forwarding address from our worker
    let fwd_route = ctx.receive::<Route>().await?.take();

    // Then send a message to our worker, via the hub
    ctx.send_message(fwd_route, "Hello you!".to_string())
        .await?;

    Ok(())
}
