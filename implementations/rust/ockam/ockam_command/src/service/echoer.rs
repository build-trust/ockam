use ockam::Context;
use ockam_core::{Result, Routed, Worker};

pub struct Echoer;

pub const ECHOER_SERVICE_NAME: &str = "ECHOER";

#[ockam::worker]
impl Worker for Echoer {
    type Message = String;
    type Context = Context;

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<String>) -> Result<()> {
        println!("Address: {}, Received: {}", ctx.address(), msg);

        // Echo the message body back on its return_route.
        ctx.send(msg.return_route(), msg.body()).await
    }
}
