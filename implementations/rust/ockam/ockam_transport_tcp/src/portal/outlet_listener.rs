use crate::{PortalMessage, TcpRouterHandle};
use ockam_core::async_trait;
use ockam_core::{route, Address, LocalMessage, Result, Routed, TransportMessage, Worker};
use ockam_node::Context;
use tracing::debug;

pub(crate) struct TcpOutletListenWorker {
    router_handle: TcpRouterHandle,
    peer: String,
}

impl TcpOutletListenWorker {
    pub(crate) async fn start(
        router_handle: &TcpRouterHandle,
        address: Address,
        peer: String,
    ) -> Result<()> {
        let worker = Self {
            router_handle: router_handle.clone(),
            peer,
        };

        router_handle.ctx().start_worker(address, worker).await?;

        Ok(())
    }
}

#[async_trait]
impl Worker for TcpOutletListenWorker {
    type Context = Context;
    type Message = PortalMessage;

    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        let address = self.router_handle.connect_outlet(self.peer.clone()).await?;

        debug!("Created Tcp Outlet at {}", &address);

        let msg = TransportMessage::v1(route![address], msg.return_route(), msg.payload().to_vec());

        ctx.forward(LocalMessage::new(msg, vec![])).await?;

        Ok(())
    }
}
