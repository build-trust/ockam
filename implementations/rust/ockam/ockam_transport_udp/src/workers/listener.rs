use super::TransportMessageCodec;
use crate::UDP;
use futures_util::stream::SplitStream;
use futures_util::StreamExt;
use ockam_core::{async_trait, route, Address, AllowAll, LocalMessage, Processor, Result};
use ockam_node::Context;
use tokio_util::udp::UdpFramed;
use tracing::{debug, warn};

/// A listener for the UDP transport
///
/// This processor handles the reception of messages on a
/// local socket. See [`UdpRouter`](crate::router::UdpRouter) for more details.
///
/// When a message is received, the address of the paired sender
/// ([`UdpSendWorker`](crate::workers::UdpSendWorker)) is injected into the message's
/// return route so that replies are sent to the sender.
pub(crate) struct UdpListenProcessor {
    /// The read half of the udnerlying UDP socket.
    stream: SplitStream<UdpFramed<TransportMessageCodec>>,
    /// Address of our sender counterpart
    sender_addr: Address,
}

impl UdpListenProcessor {
    pub(crate) async fn start(
        ctx: &Context,
        stream: SplitStream<UdpFramed<TransportMessageCodec>>,
        sender_addr: Address,
    ) -> Result<()> {
        let processor = Self {
            stream,
            sender_addr,
        };
        let addr = Address::random_tagged("UdpListenProcessor");

        // FIXME: @ac
        ctx.start_processor(addr.clone(), processor, AllowAll, AllowAll)
            .await?;

        Ok(())
    }
}

#[async_trait]
impl Processor for UdpListenProcessor {
    type Context = Context;

    async fn initialize(&mut self, ctx: &mut Context) -> Result<()> {
        ctx.set_cluster(crate::CLUSTER_NAME).await
    }

    async fn process(&mut self, ctx: &mut Self::Context) -> Result<bool> {
        debug!("Waiting for incoming UDP datagram...");
        let (mut msg, addr) = match self.stream.next().await {
            Some(res) => match res {
                Ok((msg, addr)) => (msg, addr),
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

        // Set return route to go directly to paired sender, skipping the UDP router
        msg.return_route = route![
            self.sender_addr.clone(),
            Address::new(UDP, addr.to_string()),
            msg.return_route
        ];

        debug!(onward_route = %msg.onward_route,
            return_route = %msg.return_route,
            "Forwarding UDP message");
        ctx.forward(LocalMessage::new(msg, vec![])).await?;

        Ok(true)
    }
}
