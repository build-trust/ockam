use crate::{
    pipe2::{PipeReceiver, PipeSystem},
    Context, OckamMessage,
};
use ockam_core::{compat::boxed::Box, Address, Any, Encodable, Result, Routed, Worker};

/// Listen for pipe2 handshakes and creates PipeReceiver workers
pub struct PipeListener {
    system: PipeSystem,
}

impl PipeListener {
    pub fn new(system: PipeSystem) -> Self {
        Self { system }
    }
}

#[crate::worker]
impl Worker for PipeListener {
    type Context = Context;
    type Message = OckamMessage;

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<OckamMessage>) -> Result<()> {
        // We just assume that any messages that comes in is a
        // handshake request.  We probably want to check the metadata
        // in that OckamMessage to make sure.

        // TODO: use the worker system to handle the handshake
        let (api_addr, init_addr) = (Address::random(0), Address::random(0));
        let worker = PipeReceiver::new(
            self.system.clone(),
            api_addr.clone(),
            Some(init_addr.clone()),
        );
        ctx.start_worker(vec![api_addr, init_addr.clone()], worker)
            .await?;

        // Store the return route of the request in the scope metadata section
        let ockam_msg = OckamMessage::new(Any)?.scope_data(msg.return_route().encode()?);
        ctx.send(init_addr, ockam_msg).await?;
        Ok(())
    }
}
