use crate::{PortalMessage, TcpRouterHandle};
use ockam_core::{async_trait, AsyncTryClone};
use ockam_core::{Address, Result, Routed, Worker};
use ockam_node::Context;
use ockam_transport_core::TransportError;
use tracing::debug;

/// A TCP Portal Outlet listen worker
///
/// TCP Portal Outlet listen workers are created by `TcpTransport`
/// after a call is made to
/// [`TcpTransport::create_outlet`](crate::TcpTransport::create_outlet).
pub(crate) struct TcpOutletListenWorker {
    router_handle: TcpRouterHandle,
    peer: String,
}

impl TcpOutletListenWorker {
    /// Start a new `TcpOutletListenWorker`
    pub(crate) async fn start(
        router_handle: &TcpRouterHandle,
        address: Address,
        peer: String,
    ) -> Result<()> {
        let worker = Self {
            router_handle: router_handle.async_try_clone().await?,
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
        _ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        let return_route = msg.return_route();

        if let PortalMessage::Ping = msg.body() {
        } else {
            return Err(TransportError::Protocol.into());
        }

        let address = self
            .router_handle
            .connect_outlet(self.peer.clone(), return_route.clone())
            .await?;

        debug!("Created Tcp Outlet at {}", &address);

        Ok(())
    }
}
