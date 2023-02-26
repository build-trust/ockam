use super::TransportMessageCodec;
use crate::UDP;
use futures_util::{stream::SplitSink, SinkExt};
use ockam_core::{async_trait, Any, Result, Routed, TransportMessage, Worker};
use ockam_node::Context;
use ockam_transport_core::TransportError;
use std::net::{SocketAddr, ToSocketAddrs};
use tokio_util::udp::UdpFramed;
use tracing::{error, trace, warn};

/// A sender for the UDP transport
///
/// This worker handles the sending of messages on a
/// local socket. See [`UdpRouter`](crate::router::UdpRouter) for more details.
pub(crate) struct UdpSendWorker {
    /// The read half of the udnerlying UDP socket.
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
        _ctx: &mut Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        // Parse message and remove our address from its routing
        let mut msg = msg.into_transport_message();
        msg.onward_route.step()?;

        trace!("Sending message to {:?}", msg.onward_route);

        // Resolve peer address to IPv4 SocketAddr(s).
        let peer_addr = msg.onward_route.step()?;

        if peer_addr.transport_type() != UDP {
            error!(addr = %peer_addr,
                "Destination address is not UDP");
            return Err(TransportError::UnknownRoute.into());
        }

        let peer_addr = peer_addr.address();
        let peer_addrs = peer_addr
            .to_socket_addrs()
            .map_err(|_| TransportError::InvalidAddress)?;
        let peer_addrs: Vec<_> = peer_addrs.filter(SocketAddr::is_ipv4).collect();

        // Try to send to first SocketAddr
        let addr = match peer_addrs.first() {
            Some(a) => *a,
            None => {
                warn!("No IPv4 address resolved for peer {:?}", peer_addr);
                return Err(TransportError::UnknownRoute.into());
            }
        };

        // Error on conditions that _might_ put the sink
        // into an error state
        if addr.port() == 0 {
            warn!(peer_addr = %peer_addr, "Will not send to address");
            return Err(TransportError::InvalidAddress.into());
        }

        // Send
        match self.sink.send((msg.clone(), addr)).await {
            Ok(()) => {
                trace!("Successful send to {}", addr);
                Ok(())
            }
            Err(e) => {
                error!("Failed send to {}: {:?}", addr, e);
                Err(e.into())
            }
        }
    }
}
