use super::{Addresses, UdpSocketRead};
use crate::messages::{
    RoutingNumber, UdpRoutingMessage, UdpTransportMessage, MAX_ON_THE_WIRE_SIZE,
};
use crate::UDP;
use ockam_core::compat::collections::HashMap;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{async_trait, Address, Error, LocalMessage, Processor, Result, RouteBuilder};
use ockam_node::Context;
use std::net::SocketAddr;
use tracing::{trace, warn};

/// Pending routing messages that we haven't yet assembled for all peers
/// TODO: Clearing everything for a socket after long inactivity would be nice
#[derive(Default)]
struct PendingRoutingMessageStorage(HashMap<SocketAddr, PeerPendingRoutingMessageStorage>);

/// Pending routing messages for a certain peer
/// Currently, we only support messages with the right order, which means:
///  1. If a newer routing message is received - the old one is dropped if it wasn't fully assembled
///  2. If a part of a routing message has arrived out of order - the message is fully dropped
struct PeerPendingRoutingMessageStorage {
    expected_routing_number: RoutingNumber,
    routing_message_binary: Vec<u8>,
}

impl PeerPendingRoutingMessageStorage {
    // Create given a first received message
    fn new(routing_number: RoutingNumber) -> Self {
        Self {
            expected_routing_number: routing_number,
            routing_message_binary: vec![],
        }
    }

    fn add_transport_message_and_try_assemble(
        &mut self,
        transport_message: UdpTransportMessage<'_>,
    ) -> Option<UdpRoutingMessage<'static>> {
        trace!(
            "Received routing message {}, offset {}",
            transport_message.routing_number,
            transport_message.offset
        );

        if transport_message.routing_number < self.expected_routing_number {
            warn!(
                "Dropping routing message: {} because it arrived late. Offset {}",
                transport_message.routing_number, transport_message.offset
            );
            return None;
        }

        if transport_message.routing_number > self.expected_routing_number {
            warn!(
                "Dropping routing message {} because a new routing message has arrived: {}",
                self.expected_routing_number, transport_message.routing_number
            );
            self.routing_message_binary.clear();
            self.expected_routing_number = transport_message.routing_number;
        }

        if self.routing_message_binary.len() != transport_message.offset as usize {
            warn!(
                "Dropping routing message: {} because expected offset is {}, while received offset is {}",
                transport_message.routing_number,
                self.routing_message_binary.len(),
                transport_message.offset
            );
            self.routing_message_binary.clear();
            self.expected_routing_number.increment();
            return None;
        }

        self.routing_message_binary
            .extend_from_slice(&transport_message.payload);

        if !transport_message.is_last {
            return None;
        }

        let res = match minicbor::decode::<UdpRoutingMessage>(&self.routing_message_binary) {
            Ok(routing_message) => Some(routing_message.into_owned()),
            Err(err) => {
                warn!("Error while decoding UDP message {}", err);
                None
            }
        };

        self.expected_routing_number.increment();
        self.routing_message_binary.clear();

        res
    }
}

impl PendingRoutingMessageStorage {
    fn add_transport_message_and_try_assemble(
        &mut self,
        peer: SocketAddr,
        transport_message: UdpTransportMessage<'_>,
    ) -> Option<UdpRoutingMessage<'static>> {
        let routing_number = transport_message.routing_number;

        let peer_pending_messages = self
            .0
            .entry(peer)
            .or_insert_with(|| PeerPendingRoutingMessageStorage::new(routing_number));

        peer_pending_messages.add_transport_message_and_try_assemble(transport_message)
    }
}

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
    /// Will be Some if we communicate with one specific peer.
    peer: Option<SocketAddr>,
    /// Pending routing messages that we haven't yet assembled fully
    pending_routing_messages: PendingRoutingMessageStorage,
}

impl UdpReceiverProcessor {
    pub fn new(addresses: Addresses, socket_read: UdpSocketRead, peer: Option<SocketAddr>) -> Self {
        Self {
            addresses,
            socket_read,
            peer,
            pending_routing_messages: Default::default(),
        }
    }
}

#[async_trait]
impl Processor for UdpReceiverProcessor {
    type Context = Context;

    async fn initialize(&mut self, ctx: &mut Context) -> Result<()> {
        ctx.set_cluster(crate::CLUSTER_NAME).await
    }

    async fn process(&mut self, ctx: &mut Self::Context) -> Result<bool> {
        trace!("Waiting for incoming UDP datagram...");

        let mut buf = [0u8; MAX_ON_THE_WIRE_SIZE];
        let (len, addr) = self
            .socket_read
            .recv_from(&mut buf)
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

        let transport_message: UdpTransportMessage = minicbor::decode(&buf[..len])?;

        // Let's save newly received message and see if we can assemble a Routing Message
        let routing_message = match self
            .pending_routing_messages
            .add_transport_message_and_try_assemble(addr, transport_message)
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

        trace!(onward_route = %local_message.onward_route_ref(),
            return_route = %local_message.return_route_ref(),
            "Forwarding UDP message");

        ctx.forward(local_message).await?;

        Ok(true)
    }
}
