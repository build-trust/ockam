use ockam::{Context, Result, Routed, Worker};
use tracing::Level;

pub struct Uppercase;

#[ockam::worker]
impl Worker for Uppercase {
    type Message = String;
    type Context = Context;

    #[instrument(skip_all, name = "Uppercase::handle_message", level = Level::TRACE)]
    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<String>) -> Result<()> {
        let return_route = msg.return_route().clone();
        ctx.send(return_route.clone(), msg.into_body()?.to_uppercase())
            .await
    }
}
