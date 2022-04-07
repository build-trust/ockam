use crate::{Context, Result, Routed, Worker};
use ockam_core::compat::boxed::Box;
use ockam_core::compat::string::String;

/// A worker which accepts `String`s, and echos them (and the address) to
/// the `debug!` log.
///
/// Mostly intended for use when debugging.
pub struct Echoer;

#[crate::worker]
impl Worker for Echoer {
    type Context = Context;
    type Message = String;

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<String>) -> Result<()> {
        debug!("Address: {:?}, Received: {}", ctx.address(), msg);

        // Echo the message body back on its return_route.
        ctx.send(msg.return_route(), msg.body()).await
    }
}
