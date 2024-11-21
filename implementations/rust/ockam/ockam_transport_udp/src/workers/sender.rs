use super::{Addresses, UdpSocketWrite};
use crate::messages::{
    RoutingNumber, UdpRoutingMessage, UdpTransportMessage, CURRENT_VERSION, MAX_PAYLOAD_SIZE,
};
use crate::{MAX_MESSAGE_SIZE, UDP};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{async_trait, Any, Error, LocalMessage, Result, Routed, Worker};
use ockam_node::compat::asynchronous::resolve_peer;
use ockam_node::Context;
use ockam_transport_core::TransportError;
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
}

impl UdpSenderWorker {
    /// Create a new `UdpSendWorker`
    pub(crate) fn new(
        addresses: Addresses,
        socket_write: UdpSocketWrite,
        peer: Option<SocketAddr>,
    ) -> Self {
        Self {
            addresses,
            socket_write,
            peer,
            current_routing_number: RoutingNumber::new(),
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

            // Avoid doing that each time
            resolve_peer(peer_addr.address().to_string()).await?
        };

        // Error on conditions that _might_ put the sink
        // into an error state
        if peer.port() == 0 {
            warn!(peer_addr = %peer, "Will not send to address");
            return Err(TransportError::InvalidAddress(peer.to_string()))?;
        }

        // Serialize a [`LocalMessage`] into a vector of smaller messages suitable for 1 UDP datagram
        let messages = TransportMessagesIterator::new(self.current_routing_number, msg)?;

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

struct TransportMessagesIterator {
    current_routing_number: RoutingNumber,
    offset: u32,
    data: Vec<u8>,
}

impl TransportMessagesIterator {
    fn new(current_routing_number: RoutingNumber, local_message: LocalMessage) -> Result<Self> {
        let routing_message = UdpRoutingMessage::from(local_message);

        let routing_message = ockam_core::cbor_encode_preallocate(routing_message)?;

        if routing_message.len() > MAX_MESSAGE_SIZE {
            return Err(TransportError::MessageLengthExceeded)?;
        }

        Ok(Self {
            current_routing_number,
            offset: 0,
            data: routing_message,
        })
    }
}

impl Iterator for TransportMessagesIterator {
    type Item = Result<Vec<u8>>;

    fn next(&mut self) -> Option<Self::Item> {
        let offset = self.offset as usize;
        if offset == self.data.len() {
            return None;
        }

        let mut is_last = true;
        let mut length = self.data.len() - offset;
        if length > MAX_PAYLOAD_SIZE {
            is_last = false;
            length = MAX_PAYLOAD_SIZE;
        }

        let part = UdpTransportMessage::new(
            CURRENT_VERSION,
            self.current_routing_number,
            self.offset,
            is_last,
            &self.data[offset..offset + length],
        );

        trace!(
            "Sending Routing Message {}. Offset {}",
            self.current_routing_number,
            part.offset
        );

        match ockam_core::cbor_encode_preallocate(part) {
            Ok(res) => {
                self.offset += length as u32;
                Some(Ok(res))
            }
            Err(err) => Some(Err(err)),
        }
    }
}
