use crate::{
    channel::CLUSTER_NAME,
    pipe::{PipeBehavior, PipeReceiver, PipeSender},
    protocols::channel::{ChannelCreationHandshake, ChannelProtocol},
    Context,
};
use ockam_core::{Address, Result, Routed, Worker};

pub struct ChannelListener {
    hooks: PipeBehavior,
}

impl ChannelListener {
    pub async fn create(ctx: &Context, addr: Address, hooks: PipeBehavior) -> Result<()> {
        ctx.start_worker(addr, Self { hooks }).await
    }
}

#[crate::worker]
impl Worker for ChannelListener {
    type Message = ChannelCreationHandshake;
    type Context = Context;

    async fn initialize(&mut self, ctx: &mut Context) -> Result<()> {
        ctx.set_cluster(CLUSTER_NAME).await?;
        Ok(())
    }

    async fn handle_message(
        &mut self,
        ctx: &mut Context,
        msg: Routed<ChannelCreationHandshake>,
    ) -> Result<()> {
        info!(
            "Receiving new channel creation request from {:?}",
            msg.return_route()
        );

        // First compute the route to the peer pipe receiver
        let ChannelCreationHandshake(ref rx_addr) = dbg!(msg.as_body());
        let peer = dbg!(msg.return_route().recipient());
        let rx_route = dbg!(msg
            .return_route()
            .modify()
            .pop_back()
            .append(rx_addr.clone())
            .append(peer)
            .into());

        // Start the PipeSender pointing at the remote receiver
        let tx_addr = Address::random(0);
        PipeSender::create(
            ctx,
            rx_route,
            tx_addr.clone(),
            Address::random(0),
            self.hooks.clone(),
        )
        .await?;
        debug!("Started PipeSender with appropriate sender route");

        // Start the PipeReceiver
        let rx_addr = Address::random(0);
        PipeReceiver::create(ctx, rx_addr.clone(), Address::random(0), self.hooks.clone()).await?;

        // Create the local channel worker

        // Then message the remote Channel peer
        ctx.send(msg.return_route(), ChannelProtocol::ReceiverReady(rx_addr))
            .await?;

        Ok(())
    }
}
