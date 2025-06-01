use ockam::{Context, Result, Routed, Worker};

pub struct Echoer;

#[ockam::worker]
impl Worker for Echoer {
    type Context = Context;
    type Message = String;

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<String>) -> Result<()> {
        println!("Address: {}, Received: {:?}", ctx.primary_address(), msg);

        // Echo the message body back on its return_route.
        ctx.send(msg.return_route().clone(), msg.into_body()?).await
    }
}
