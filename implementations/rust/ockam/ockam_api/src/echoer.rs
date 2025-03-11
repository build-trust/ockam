use ockam::{Any, Context, Result, Routed, Worker};
use ockam_core::NeutralMessage;
use tracing as log;
use tracing::Level;

pub struct Echoer;

#[ockam::worker]
impl Worker for Echoer {
    type Context = Context;
    type Message = Any;

    #[instrument(skip_all, name = "Echoer::handle_message", level = Level::TRACE)]
    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<Any>) -> Result<()> {
        log::debug!(src = %msg.src_addr(), from = %msg.sender()?, to = %msg.return_route().next()?, "echoing back");
        let msg = msg.into_local_message();
        ctx.send(msg.return_route, NeutralMessage::from(msg.payload))
            .await
    }
}
