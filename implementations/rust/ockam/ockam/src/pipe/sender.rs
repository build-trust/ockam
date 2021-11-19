use crate::{
    monotonic::Monotonic,
    pipe::PipeBehavior,
    protocols::pipe::{internal::InternalCmd, PipeMessage},
};
use ockam_core::compat::boxed::Box;
use ockam_core::{Address, Any, Result, Route, Routed, Worker};
use ockam_node::Context;

pub struct PipeSender {
    peer: Route,
    int_addr: Address,
    index: Monotonic,
    hooks: PipeBehavior,
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
        hooks: PipeBehavior,
    ) -> Result<()> {
        ctx.start_worker(
            vec![addr, int_addr.clone()],
            PipeSender {
                peer,
                int_addr,
                // Ordered pipes expect a 1-indexed message
                index: Monotonic::from(1),
                hooks,
            },
        )
        .await
    }

    /// Handle internal command payloads
    async fn handle_internal(&mut self, ctx: &mut Context, msg: Routed<Any>) -> Result<()> {
        trace!("PipeSender receiving internal command");
        let trans = msg.into_transport_message();
        let internal_cmd = InternalCmd::from_transport(&trans)?;
        self.hooks
            .internal_all(self.int_addr.clone(), self.peer.clone(), ctx, &internal_cmd)
            .await
    }

    /// Handle external user messages
    async fn handle_external(&mut self, ctx: &mut Context, msg: Routed<Any>) -> Result<()> {
        // First manipulate the onward_route state
        let mut msg = msg.into_transport_message();
        msg.onward_route.modify().pop_front();

        debug!(
            "Pipe sender dispatch {:?} -> {:?}",
            self.peer, msg.onward_route
        );

        // Then pack TransportMessage into PipeMessage
        let index = self.index.next() as u64;
        let pipe_msg = PipeMessage::from_transport(index, msg)?;

        // Before we send we give all hooks a chance to run
        if let crate::pipe::PipeModifier::Drop = self
            .hooks
            .external_all(self.int_addr.clone(), self.peer.clone(), ctx, &pipe_msg)
            .await?
        {
            // Return early to prevent message sending if the
            // behaviour stack has determined to drop the message.
            return Ok(());
        }

        // Then send the message from our internal address so the
        // receiver can send any important messages there
        ctx.send_from_address(self.peer.clone(), pipe_msg, self.int_addr.clone())
            .await
    }
}
