use ockam::{Context, Result, Routed, Worker};

pub struct Uppercase;

#[ockam::worker]
impl Worker for Uppercase {
    type Message = String;
    type Context = Context;

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<String>) -> Result<()> {
        ctx.send(msg.return_route(), msg.body().to_uppercase())
            .await
    }
}
