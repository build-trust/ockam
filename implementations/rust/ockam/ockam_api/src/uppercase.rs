use ockam::{Context, Result, Routed, Worker};

pub struct Uppercase;

#[ockam::worker]
impl Worker for Uppercase {
    type Message = String;
    type Context = Context;

    #[instrument(skip_all, name = "Uppercase::handle_message")]
    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<String>) -> Result<()> {
        ctx.send(msg.return_route().clone(), msg.into_body()?.to_uppercase())
            .await
    }
}
