use super::{Addresses, TransportMessageCodec};
use crate::UDP;
use futures_util::{stream::SplitSink, SinkExt};
use ockam_core::{async_trait, Any, Result, Routed, TransportMessage, Worker};
use ockam_node::Context;
use ockam_transport_core::{resolve_peer, TransportError};
use std::net::SocketAddr;
use tokio_util::udp::UdpFramed;
use tracing::{error, trace, warn};

/// A sender for the UDP transport
///
/// This worker handles the sending of messages on a
/// local socket. See [`UdpRouter`](crate::router::UdpRouter) for more details.
pub(crate) struct UdpSenderWorker {
    addresses: Addresses,
    /// The read half of the underlying UDP socket.
    sink: SplitSink<UdpFramed<TransportMessageCodec>, (TransportMessage, SocketAddr)>,
    /// Will be Some if we communicate with one specific peer.
    peer: Option<SocketAddr>,
}

impl UdpSenderWorker {
    /// Create a new `UdpSendWorker`
    pub(crate) fn new(
        addresses: Addresses,
        sink: SplitSink<UdpFramed<TransportMessageCodec>, (TransportMessage, SocketAddr)>,
        peer: Option<SocketAddr>,
    ) -> Self {
        Self {
            addresses,
            sink,
            peer,
        }
    }
}

#[async_trait]
impl Worker for UdpSenderWorker {
    type Message = Any;
    type Context = Context;

    async fn shutdown(&mut self, ctx: &mut Self::Context) -> Result<()> {
        let _ = ctx
            .stop_processor(self.addresses.receiver_address().clone())
            .await;

        Ok(())
    }

    async fn handle_message(
        &mut self,
        _ctx: &mut Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        // Parse message and remove our address from its routing
        let mut msg = msg.into_local_message();
        msg = msg.pop_front_onward_route()?;
        trace!("Sending message to {:?}", msg.onward_route_ref());

        let peer = if let Some(peer) = &self.peer {
            *peer
        } else {
            // Resolve peer address to IPv4 SocketAddr(s).
            let peer_addr = msg.next_on_onward_route()?;
            msg = msg.pop_front_onward_route()?;

            if peer_addr.transport_type() != UDP {
                error!(addr = %peer_addr,
                "Destination address is not UDP");
                return Err(TransportError::UnknownRoute)?;
            }

            resolve_peer(peer_addr.address().to_string())?
        };

        // Error on conditions that _might_ put the sink
        // into an error state
        if peer.port() == 0 {
            warn!(peer_addr = %peer, "Will not send to address");
            return Err(TransportError::InvalidAddress(peer.to_string()))?;
        }

        // Send
        match self.sink.send((msg.into_transport_message(), peer)).await {
            Ok(()) => {
                trace!("Successful send to {}", peer);
                Ok(())
            }
            Err(e) => {
                error!("Failed send to {}: {:?}", peer, e);
                Err(e)?
            }
        }
    }
}
