//! You need to start this binary first, before letting the
//! `client` connect to it.

#[macro_use]
extern crate tracing;

use ockam::{worker, Context, Result, Routed, Worker};

use ockam_transport_websocket::WebSocketTransport;

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Get either the default socket address, or a user-input
    let bind_addr = get_bind_addr();
    let ws = WebSocketTransport::create(&ctx).await?;
    ws.listen(bind_addr).await?;

    // Create the responder worker
    ctx.start_worker("echoer", Responder).await?;

    // The server never shuts down
    Ok(())
}

struct Responder;

#[worker]
impl Worker for Responder {
    type Context = Context;
    type Message = String;

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<String>) -> Result<()> {
        info!("Message: {}", msg);
        debug!("Replying back to {}", &msg.return_route());
        ctx.send(msg.return_route(), msg.body()).await?;
        Ok(())
    }
}

fn get_bind_addr() -> String {
    std::env::args()
        .skip(1)
        .take(1)
        .next()
        .unwrap_or_else(|| "127.0.0.1:10222".to_string())
}
