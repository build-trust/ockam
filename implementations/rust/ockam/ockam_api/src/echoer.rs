use ockam::{Any, Context, Result, Routed, Worker};
use ockam_core::NeutralMessage;
use tracing as log;

pub struct Echoer;

#[ockam::worker]
impl Worker for Echoer {
    type Context = Context;
    type Message = Any;

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<Any>) -> Result<()> {
        log::debug!(src = %msg.src_addr(), from = %msg.sender()?, to = %msg.return_route().step()?, "echoing back");
        ctx.send(msg.return_route(), NeutralMessage::from(msg.take_payload()))
            .await
    }
}
