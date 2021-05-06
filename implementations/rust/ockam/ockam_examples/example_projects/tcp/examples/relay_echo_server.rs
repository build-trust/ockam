#[macro_use]
extern crate tracing;

use ockam::{Context, Result, Route, Routed, Worker};
use ockam_transport_tcp::{TcpTransport, TCP};

fn get_peer_addr() -> String {
    std::env::args()
        .skip(1)
        .take(1)
        .next()
        // This value can be used when running the ockam-hub locally
        .unwrap_or(format!("127.0.0.1:4000"))
}

/// A worker who isn't always available.  It registers itself with the
/// hub forwarding service to never miss a message.
struct ProxiedWorker {
    /// The hub's address
    peer: String,
}

#[ockam::worker]
impl Worker for ProxiedWorker {
    type Context = Context;
    type Message = String;

    async fn initialize(&mut self, ctx: &mut Context) -> Result<()> {
        // Register this service with the hub's forwarding service
        ctx.send(
            Route::new()
                .append_t(TCP, &self.peer)
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
    let tcp = TcpTransport::create(&ctx).await?;
    tcp.connect(peer.clone()).await?;

    // Start the worker we want to reach via proxy
    ctx.start_worker("worker", ProxiedWorker { peer }).await?;

    Ok(())
}
