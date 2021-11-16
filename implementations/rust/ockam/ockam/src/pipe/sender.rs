use ockam_core::{Address, Any, Result, Route, Routed, Worker};
use ockam_node::Context;

pub struct PipeSender {
    peer: Route,
    int_addr: Address,
}

#[ockam_core::worker]
impl Worker for PipeSender {
    type Context = Context;
    type Message = Any;

    async fn initialize(&mut self, ctx: &mut Context) -> Result<()> {
        ctx.set_cluster("_internal.messaging.pipe").await?;
        Ok(())
    }

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<Any>) -> Result<()> {
        match msg.msg_addr() {
            addr if addr == self.int_addr => self.handle_internal(ctx, msg).await?,
            _ => self.handle_external(ctx, msg).await?,
        };

        Ok(())
    }
}

impl PipeSender {
    /// Create a PipeSender with a public and "internal" address
    pub async fn create(
        ctx: &mut Context,
        peer: Route,
        addr: Address,
        int_addr: Address,
    ) -> Result<()> {
        ctx.start_worker(vec![addr, int_addr.clone()], PipeSender { peer, int_addr })
            .await
    }

    /// Handle internal command payloads
    async fn handle_internal(&mut self, _ctx: &mut Context, _msg: Routed<Any>) -> Result<()> {
        debug!("PipeSender handling internal command");
        Ok(())
    }

    /// Handle external user messages
    async fn handle_external(&mut self, ctx: &mut Context, msg: Routed<Any>) -> Result<()> {
        let mut msg = msg.into_local_message();
        msg.transport_mut()
            .onward_route
            .modify()
            .pop_front()
            .prepend_route(self.peer.clone());
        debug!(
            "Pipe sender forwarding message to {:?}",
            msg.transport().onward_route
        );
        ctx.forward(msg).await?;
        Ok(())
    }
}
