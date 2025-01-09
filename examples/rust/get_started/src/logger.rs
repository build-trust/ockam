use ockam::{Any, Context, Result, Routed, Worker};

pub struct Logger;

#[ockam::worker]
impl Worker for Logger {
    type Context = Context;
    type Message = Any;

    /// This handle function takes any incoming message and print its content as a UTF-8 string
    /// if it's correct UTF-8 string, and in hex if it's not. After that, Worker forwards
    /// the message to the next hop in it's onward route
    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<Any>) -> Result<()> {
        let local_msg = msg.into_local_message();
        let payload = local_msg.payload();

        if let Ok(str) = String::from_utf8(payload.to_vec()) {
            println!("Address: {}, Received string: {}", ctx.primary_address(), str);
        } else {
            println!(
                "Address: {}, Received binary: {}",
                ctx.primary_address(),
                hex::encode(payload)
            );
        }

        ctx.forward(local_msg.step_forward(ctx.primary_address().clone())?)
            .await
    }
}
