use ockam::{Context, Result, Routed, Worker};
use tracing::info;

pub struct Server;

#[ockam::worker]
impl Worker for Server {
    type Context = Context;
    type Message = String;

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<String>) -> Result<()> {
        let return_route = msg.return_route();
        let msg_str = msg.body();
        info!("Server received message: {}", msg_str);

        ctx.send(return_route, msg_str.clone()).await?;
        info!("Server sent message: {}", msg_str);

        Ok(())
    }
}
