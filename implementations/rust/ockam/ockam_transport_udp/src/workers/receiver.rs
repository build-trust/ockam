use super::{Addresses, UdpSocketRead};
use crate::messages::UdpTransportMessage;
use crate::workers::pending_messages::PendingRoutingMessageStorage;
use crate::UDP;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{async_trait, Address, Error, LocalMessage, Processor, Result, RouteBuilder};
use ockam_node::Context;
use std::net::SocketAddr;
use tracing::{trace, warn};

/// A listener for the UDP transport
///
/// This processor handles the reception of messages on a
/// local socket.
///
/// When a message is received, the address of the paired sender
/// ([`UdpSendWorker`](crate::workers::UdpSenderWorker)) is injected into the message's
/// return route so that replies are sent to the sender.
pub(crate) struct UdpReceiverProcessor {
    addresses: Addresses,
    /// The read half of the underlying UDP socket.
    socket_read: UdpSocketRead,
    buffer: Vec<u8>,
    /// Will be Some if we communicate with one specific peer.
    peer: Option<SocketAddr>,
    /// Pending routing messages that we haven't yet assembled fully
    pending_routing_messages: PendingRoutingMessageStorage,
    max_on_the_wire_packet_size: usize,
}

impl UdpReceiverProcessor {
    pub fn new(
        addresses: Addresses,
        socket_read: UdpSocketRead,
        peer: Option<SocketAddr>,
        max_pending_messages_per_peer: u16,
        max_on_the_wire_packet_size: usize,
    ) -> Self {
        Self {
            addresses,
            socket_read,
            buffer: vec![0; max_on_the_wire_packet_size],
            peer,
            pending_routing_messages: PendingRoutingMessageStorage::new(
                max_pending_messages_per_peer,
            ),
            max_on_the_wire_packet_size,
        }
    }
}

#[async_trait]
impl Processor for UdpReceiverProcessor {
    type Context = Context;

    async fn process(&mut self, ctx: &mut Self::Context) -> Result<bool> {
        trace!("Waiting for incoming UDP datagram...");

        self.buffer.clear();
        self.buffer.resize(self.max_on_the_wire_packet_size, 0);

        let (len, addr) = self
            .socket_read
            .recv_from(&mut self.buffer)
            .await
            .map_err(|e| Error::new(Origin::Transport, Kind::Io, e))?;

        if let Some(peer) = &self.peer {
            if peer != &addr {
                warn!(
                    "Dropping a packet from: {}, because expected address was: {}",
                    addr, peer
                );
                // Drop the packet, we don't expect data from that peer
                return Ok(true);
            }
        }

        let transport_message: UdpTransportMessage = minicbor::decode(&self.buffer[..len])?;

        // Let's save newly received message and see if we can assemble a Routing Message
        let routing_message = match self
            .pending_routing_messages
            .add_transport_message_and_try_assemble(addr, transport_message)?
        {
            Some(routing_message) => routing_message,
            None => {
                // We need more data to assemble a routing message
                return Ok(true);
            }
        };

        if routing_message.onward_route.is_empty() {
            return Ok(true);
        }

        let return_route = RouteBuilder::default().append(self.addresses.sender_address().clone());

        let return_route = if self.peer.is_some() {
            // If the peer address is defined, we don't need to specify it in the return route
            return_route
        } else {
            // Add the peer address so that sender knows where to send the message
            return_route.append(Address::new_with_string(UDP, addr.to_string()))
        };

        let mut local_message = LocalMessage::from(routing_message);

        let return_route = return_route.append_route(local_message.return_route.clone());

        local_message = local_message.set_return_route(return_route.into());

        trace!(onward_route = %local_message.onward_route(),
            return_route = %local_message.return_route(),
            "Forwarding UDP message");

        ctx.forward(local_message).await?;

        Ok(true)
    }
}
