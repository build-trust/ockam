use std::{net::SocketAddr, ops::Deref};

use futures_util::{stream::SplitSink, SinkExt};
use ockam_core::{
    async_trait, Any, Decodable, LocalMessage, Result, Routed, TransportMessage, Worker,
};
use ockam_node::Context;
use ockam_transport_core::TransportError;
use tokio_util::udp::UdpFramed;
use tracing::warn;

use crate::router::UdpRouterHandle;

use super::TransportMessageCodec;

/// A UDP message sending worker
///
/// This worker is created when `UdpTransport::listen` is called.
/// When auto connection is enabled, this work can be created
/// automatically by the router.
pub(crate) struct UdpSendWorker {
    sink: SplitSink<UdpFramed<TransportMessageCodec>, (TransportMessage, SocketAddr)>,
}

impl UdpSendWorker {
    /// Create a new `UdpSendWorker`
    pub(crate) fn new(
        sink: SplitSink<UdpFramed<TransportMessageCodec>, (TransportMessage, SocketAddr)>,
    ) -> Self {
        Self { sink }
    }
}

#[async_trait]
impl Worker for UdpSendWorker {
    type Message = Any;
    type Context = Context;

    async fn handle_message(
        &mut self,
        ctx: &mut Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        let mut msg = LocalMessage::decode(msg.payload())?.into_transport_message();

        // Remove sender address
        msg.onward_route.step()?;

        let (peer_addr, _) = match String::from_utf8(msg.onward_route.step()?.deref().clone()) {
            Ok(s) => UdpRouterHandle::resolve_peer(s)?,
            Err(_e) => return Err(TransportError::UnknownRoute.into()),
        };

        if self.sink.send((msg, peer_addr)).await.is_err() {
            warn!("Failed to send message to peer {}", peer_addr);
            ctx.stop_worker(ctx.address()).await?;
        }

        Ok(())
    }
}
