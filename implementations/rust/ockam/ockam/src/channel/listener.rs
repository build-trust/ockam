use crate::{
    channel::{worker::ChannelWorker, CLUSTER_NAME},
    pipe::PipeBehavior,
    protocols::channel::ChannelCreationHandshake,
    Context,
};
use ockam_core::{Address, Result, Routed, Worker};

pub struct ChannelListener {
    tx_hooks: PipeBehavior,
    rx_hooks: PipeBehavior,
}

impl ChannelListener {
    pub async fn create(
        ctx: &Context,
        addr: Address,
        tx_hooks: PipeBehavior,
        rx_hooks: PipeBehavior,
    ) -> Result<()> {
        ctx.start_worker(addr, Self { tx_hooks, rx_hooks }).await
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

        // First compute routes to the peer's PipeSender and PipeReceiver
        let ChannelCreationHandshake(ref rx_addr, ref tx_addr) = msg.as_body();
        let peer_rx_route = msg
            .return_route()
            .modify()
            .pop_back()
            .append(rx_addr.clone())
            .into();
        let peer_tx_route = msg
            .return_route()
            .modify()
            .pop_back()
            .append(tx_addr.clone())
            .into();

        ChannelWorker::stage2(
            ctx,
            peer_tx_route,
            peer_rx_route,
            self.tx_hooks.clone(),
            self.rx_hooks.clone(),
        )
        .await?;
        Ok(())
    }
}
