use ockam_core::{Address, Any, Result, Routed, Worker};
use ockam_node::Context;

pub struct PipeReceiver;

#[crate::worker]
impl Worker for PipeReceiver {
    type Context = Context;
    type Message = Any;

    async fn initialize(&mut self, ctx: &mut Context) -> Result<()> {
        ctx.set_cluster(super::CLUSTER_NAME).await?;
        Ok(())
    }

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<Any>) -> Result<()> {
        self.handle_external(ctx, msg).await?;
        Ok(())
    }
}

impl PipeReceiver {
    pub async fn create(ctx: &mut Context, addr: Address) -> Result<()> {
        ctx.start_worker(addr, PipeReceiver).await
    }

    /// Handle external user messages
    async fn handle_external(&mut self, ctx: &mut Context, msg: Routed<Any>) -> Result<()> {
        let mut msg = msg.into_local_message();
        msg.transport_mut().onward_route.modify().pop_front();
        debug!(
            "Pipe sender forwarding message to {:?}",
            msg.transport().onward_route
        );
        ctx.forward(msg).await?;
        Ok(())
    }
}
