use ockam::{async_worker, Context, Result, Routed, Worker};
use ockam_channel::ChannelMessage;
use tracing::info;

pub struct Server;

#[async_worker]
impl Worker for Server {
    type Context = Context;
    type Message = String;

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<String>) -> Result<()> {
        let return_route = msg.reply();
        let msg_str = msg.take();
        info!("Server received message: {}", msg_str);

        ctx.send_message(return_route, ChannelMessage::encrypt(msg_str.clone())?)
            .await?;
        info!("Server sent message: {}", msg_str);

        Ok(())
    }
}
