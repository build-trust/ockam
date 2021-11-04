use crate::{PortalMessage, TcpRouterHandle};
use ockam_core::{async_trait, AsyncTryClone, NodeContext};
use ockam_core::{route, Address, LocalMessage, Result, Routed, TransportMessage, Worker};
use tracing::debug;

pub(crate) struct TcpOutletListenWorker<C> {
    router_handle: TcpRouterHandle<C>,
    peer: String,
}

impl<C: NodeContext> TcpOutletListenWorker<C> {
    pub(crate) async fn start(
        router_handle: &TcpRouterHandle<C>,
        address: Address,
        peer: String,
    ) -> Result<()> {
        let worker = Self {
            router_handle: router_handle.async_try_clone().await?,
            peer,
        };

        router_handle
            .ctx()
            .start_worker(address.into(), worker)
            .await?;

        Ok(())
    }
}

#[async_trait]
impl<C: NodeContext> Worker<C> for TcpOutletListenWorker<C> {
    type Message = PortalMessage;

    async fn handle_message(&mut self, ctx: &mut C, msg: Routed<Self::Message>) -> Result<()> {
        let address = self.router_handle.connect_outlet(self.peer.clone()).await?;

        debug!("Created Tcp Outlet at {}", &address);

        let msg = TransportMessage::v1(route![address], msg.return_route(), msg.payload().to_vec());

        ctx.forward(LocalMessage::new(msg, vec![])).await?;

        Ok(())
    }
}
