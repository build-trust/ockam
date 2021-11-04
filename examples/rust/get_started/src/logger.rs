use ockam::{hex, Any, NodeContext, Result, Routed, Worker};

pub struct Logger;

#[ockam::worker]
impl<C: NodeContext> Worker<C> for Logger {
    type Message = Any;

    /// This handle function takes any incoming message and print its content as a UTF-8 string
    /// if it's correct UTF-8 string, and in hex if it's not. After that, Worker forwards
    /// the message to the next hop in it's onward route
    async fn handle_message(&mut self, ctx: &mut C, msg: Routed<Any>) -> Result<()> {
        let mut local_msg = msg.into_local_message();
        let transport_msg = local_msg.transport_mut();
        transport_msg.onward_route.step()?;
        transport_msg.return_route.modify().prepend(ctx.address());

        let payload = transport_msg.payload.clone();

        if let Ok(str) = String::from_utf8(payload.clone()) {
            println!("Address: {}, Received string: {}", ctx.address(), str);
        } else {
            println!("Address: {}, Received binary: {}", ctx.address(), hex::encode(&payload));
        }

        ctx.forward(local_msg).await
    }
}
