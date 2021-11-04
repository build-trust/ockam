#[cfg(all(not(feature = "std"), feature = "cortexm"))]
use ockam::{
    compat::{
        boxed::Box,
        string::String
    },
    println,
};
use ockam::{NodeContext, Result, Routed, Worker};

pub struct Echoer;

#[ockam::worker]
impl<C: NodeContext> Worker<C> for Echoer {
    type Message = String;

    async fn handle_message(&mut self, ctx: &mut C, msg: Routed<String>) -> Result<()> {
        println!("Address: {}, Received: {}", ctx.address(), msg);

        // Echo the message body back on its return_route.
        ctx.send(msg.return_route(), msg.body()).await
    }
}
