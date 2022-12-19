use crate::{
    channel::{worker::ChannelWorker, CLUSTER_NAME},
    pipe::PipeBehavior,
    protocols::channel::ChannelCreationHandshake,
    Context,
};
use ockam_core::compat::boxed::Box;
use ockam_core::{Address, AllowAll, Result, Route, Routed, Worker};

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
        ctx.start_worker(
            addr,
            Self { tx_hooks, rx_hooks },
            AllowAll, // FIXME: @ac
            AllowAll, // FIXME: @ac
        )
        .await
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
            "Receiving new channel creation request from {:?}: {:?}",
            msg.return_route(),
            msg.as_body(),
        );

        // First compute routes to the peer's PipeSender and
        // PipeReceiver with both their public and internal addresses
        let ChannelCreationHandshake {
            channel_addr, // this is the channel's internal address!
            tx_addr,
            rx_addr,
            rx_int_addr,
            tx_int_addr,
        } = msg.as_body();
        let peer_channel_addr = msg.return_route().recipient();

        let peer_rx_base: Route = msg.return_route().modify().pop_back().into();
        let peer_rx_pub = peer_rx_base.clone().modify().append(rx_addr.clone()).into();
        let peer_rx_int = peer_rx_base
            .clone()
            .modify()
            .append(rx_int_addr.clone())
            .into();

        let peer_tx_base: Route = msg.return_route().modify().pop_back().into();
        let peer_tx_pub = peer_tx_base.clone().modify().append(tx_addr.clone()).into();
        let peer_tx_int = peer_tx_base
            .clone()
            .modify()
            .append(tx_int_addr.clone())
            .into();
        ChannelWorker::stage2(
            ctx,
            (peer_tx_pub, peer_tx_int),
            (peer_rx_pub, peer_rx_int),
            channel_addr.clone(),
            peer_channel_addr,
            self.tx_hooks.clone(),
            self.rx_hooks.clone(),
        )
        .await?;
        Ok(())
    }
}
