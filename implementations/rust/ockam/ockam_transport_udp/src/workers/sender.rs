use super::{Addresses, UdpSocketWrite};
use crate::messages::{RoutingNumber, UdpRoutingMessage};
use crate::workers::pending_messages::TransportMessagesIterator;
use crate::UDP;
use core::str::FromStr;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{async_trait, Any, Error, Result, Routed, Worker};
use ockam_node::compat::asynchronous::resolve_peer;
use ockam_node::Context;
use ockam_transport_core::{HostnamePort, TransportError};
use std::net::SocketAddr;
use tracing::{error, trace, warn};

/// A sender for the UDP transport
///
/// This worker handles the sending of messages on a
/// local socket. See [`UdpRouter`](crate::router::UdpRouter) for more details.
pub(crate) struct UdpSenderWorker {
    addresses: Addresses,
    /// The read half of the underlying UDP socket.
    socket_write: UdpSocketWrite,
    /// Will be Some if we communicate with one specific peer.
    peer: Option<SocketAddr>,
    /// Current number of the packet
    current_routing_number: RoutingNumber,
    max_payload_size_per_packet: usize,
}

impl UdpSenderWorker {
    /// Create a new `UdpSendWorker`
    pub(crate) fn new(
        addresses: Addresses,
        socket_write: UdpSocketWrite,
        peer: Option<SocketAddr>,
        max_payload_size_per_packet: usize,
    ) -> Self {
        Self {
            addresses,
            socket_write,
            peer,
            current_routing_number: RoutingNumber::default(),
            max_payload_size_per_packet,
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
        trace!("Sending message to {:?}", msg.onward_route());

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

            resolve_peer(&HostnamePort::from_str(peer_addr.address())?).await?
        };

        // Error on conditions that _might_ put the sink
        // into an error state
        if peer.port() == 0 {
            warn!(peer_addr = %peer, "Will not send to address");
            return Err(TransportError::InvalidAddress(peer.to_string()))?;
        }

        // Serialize a [`LocalMessage`] into a vector of smaller messages suitable for 1 UDP datagram
        let messages = TransportMessagesIterator::new(
            self.current_routing_number,
            &UdpRoutingMessage::from(msg),
            self.max_payload_size_per_packet,
        )?;

        self.current_routing_number.increment();

        for message in messages {
            let message = message?;
            match self.socket_write.send_to(&message, peer).await {
                Ok(_) => {
                    trace!("Successful send to {}", peer);
                }
                Err(e) => {
                    error!("Failed send to {}: {:?}", peer, e);
                    return Err(Error::new(Origin::Transport, Kind::Io, e))?;
                }
            }
        }

        Ok(())
    }
}
