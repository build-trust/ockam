use ockam_core::{DenyAll, Result, Routed, Worker};
use ockam_node::Context;
use ockam_transport_udp::UdpTransport;
use tracing::debug;

#[ockam_macros::node]
async fn main(ctx: Context) -> Result<()> {
    let udp = UdpTransport::create(&ctx).await?;
    udp.listen("127.0.0.1:8000").await?;
    ctx.start_worker("echoer", Echoer, DenyAll, DenyAll).await?;
    Ok(())
}
pub struct Echoer;

#[ockam_core::worker]
impl Worker for Echoer {
    type Message = String;
    type Context = Context;

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<String>) -> Result<()> {
        debug!("Replying back to {}", &msg.return_route());
        ctx.send(msg.return_route(), msg.body()).await
    }
}
