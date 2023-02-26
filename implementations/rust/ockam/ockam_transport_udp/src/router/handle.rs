use crate::router::messages::{UdpRouterRequest, UdpRouterResponse};
use ockam_core::{Address, AllowAll, Result};
use ockam_node::Context;
use std::net::SocketAddr;

/// A handle to connect to a UdpRouter
///
/// Dropping this handle is harmless.
pub(crate) struct UdpRouterHandle {
    ctx: Context,
    api_addr: Address,
}

impl UdpRouterHandle {
    pub async fn try_new(ctx: &Context, api_addr: &Address) -> Result<Self> {
        // FIXME: @ac. The handle will only ever need to send & receive messages
        // to & from the router.
        let handle_ctx = ctx
            .new_detached(
                Address::random_tagged("UdpRouterHandle.detached"),
                AllowAll,
                AllowAll,
            )
            .await?;

        Ok(Self {
            ctx: handle_ctx,
            api_addr: api_addr.clone(),
        })
    }

    /// Request router start listening on a local UDP port
    /// so the local node can act as a server to other nodes
    pub async fn listen(&self, local_addr: SocketAddr) -> Result<()> {
        let msg = UdpRouterRequest::Listen { local_addr };
        let UdpRouterResponse::Listen(res) = self
            .ctx
            .send_and_receive(self.api_addr.clone(), msg)
            .await?;
        res
    }
}
