use crate::{
    pipe::{PipeBehavior, PipeModifier},
    protocols::pipe::PipeMessage,
};
use ockam_core::{Address, LocalMessage, Result, Routed, Worker};
use ockam_node::Context;

pub struct PipeReceiver {
    hooks: PipeBehavior,
}

#[crate::worker]
impl Worker for PipeReceiver {
    type Context = Context;
    type Message = PipeMessage;

    async fn initialize(&mut self, ctx: &mut Context) -> Result<()> {
        ctx.set_cluster(super::CLUSTER_NAME).await?;
        Ok(())
    }

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<PipeMessage>) -> Result<()> {
        self.handle_external(ctx, msg).await?;
        Ok(())
    }
}

impl PipeReceiver {
    pub async fn create(ctx: &mut Context, addr: Address, hooks: PipeBehavior) -> Result<()> {
        ctx.start_worker(addr, PipeReceiver { hooks }).await
    }

    /// Handle external user messages
    async fn handle_external(&mut self, ctx: &mut Context, msg: Routed<PipeMessage>) -> Result<()> {
        debug!("Received pipe message with index '{}'", msg.index.u64());

        // First run receiver hooks
        let return_route = msg.return_route().clone();
        let pipe_msg = msg.body();

        // Before we send we give all hooks a chance to run
        match self
            .hooks
            .external_all(ctx.address(), return_route, ctx, &pipe_msg)
            .await
        {
            // Return early to prevent message sending if the
            // behaviour stack has determined to drop the message.
            Ok(PipeModifier::Drop) => return Ok(()),
            // On errors: don't crash the relay
            Err(e) => {
                warn!("Received message was invalid: {}!", e);
                return Ok(());
            }
            // Otherwise do nothing :)
            Ok(PipeModifier::None) => {}
        }

        // If we reach this point we can safely unpack and forward
        let nested = PipeMessage::to_transport(&pipe_msg)?;
        debug!("Forwarding message to {:?}", nested.onward_route);
        ctx.forward(LocalMessage::new(nested, vec![])).await
    }
}
