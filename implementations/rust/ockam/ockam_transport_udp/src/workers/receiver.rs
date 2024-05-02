use super::{Addresses, TransportMessageCodec};
use crate::UDP;
use futures_util::stream::SplitStream;
use futures_util::StreamExt;
use ockam_core::{async_trait, Address, LocalMessage, Processor, Result, RouteBuilder};
use ockam_node::Context;
use std::net::SocketAddr;
use tokio_util::udp::UdpFramed;
use tracing::{debug, warn};

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
    stream: SplitStream<UdpFramed<TransportMessageCodec>>,
    /// Will be Some if we communicate with one specific peer.
    peer: Option<SocketAddr>,
}

impl UdpReceiverProcessor {
    pub fn new(
        addresses: Addresses,
        stream: SplitStream<UdpFramed<TransportMessageCodec>>,
        peer: Option<SocketAddr>,
    ) -> Self {
        Self {
            addresses,
            stream,
            peer,
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
        debug!("Waiting for incoming UDP datagram...");
        let (mut msg, addr) = match self.stream.next().await {
            Some(res) => match res {
                Ok((msg, addr)) => (LocalMessage::from_transport_message(msg), addr),
                Err(e) => {
                    warn!(
                        "Failed to read message, will wait for next message: {:?}",
                        e
                    );
                    return Ok(true);
                }
            },
            None => {
                debug!("No message read, will wait for next message.");
                return Ok(true);
            }
        };

        if msg.onward_route_ref().is_empty() {
            return Ok(true);
        }

        let return_route = RouteBuilder::default().append(self.addresses.sender_address().clone());

        let return_route = match &self.peer {
            Some(peer) => {
                if peer != &addr {
                    warn!(
                        "Dropping a packet from: {}, because expected address was: {}",
                        addr, peer
                    );
                    // Drop the packet, we don't expect data from that peer
                    return Ok(true);
                }

                return_route
            }
            None => return_route.append(Address::new(UDP, addr.to_string())),
        };

        let return_route = return_route.append_route(msg.return_route());

        msg = msg.set_return_route(return_route.into());

        debug!(onward_route = %msg.onward_route_ref(),
            return_route = %msg.return_route_ref(),
            "Forwarding UDP message");

        ctx.forward(msg).await?;

        Ok(true)
    }
}
