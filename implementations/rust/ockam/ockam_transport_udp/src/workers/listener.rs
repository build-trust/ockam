use futures_util::stream::SplitStream;
use futures_util::StreamExt;
use ockam_core::compat::sync::Arc;
use ockam_core::{
    async_trait, Address, AllowAll, LocalMessage, Mailbox, Mailboxes, Processor, Result,
};
use ockam_node::{Context, ProcessorBuilder};
use tokio_util::udp::UdpFramed;
use tracing::{debug, info};

use crate::{router::UdpRouterHandle, transport::UdpAddress};

use super::TransportMessageCodec;

/// A UDP listen processor
///
/// UDP listen processors are created by `UdpTransport`
/// after a call is made to
/// [`UdpTransport::listen`](crate::UdpTransport::listen).
pub(crate) struct UdpListenProcessor {
    /// The read half of the udnerlying UDP socket.
    stream: SplitStream<UdpFramed<TransportMessageCodec>>,
    /// The address of the sender worker which owns
    /// the write half of the underlying UDP socket.
    tx_addr: Address,
    /// Handle of a registered UDP router.
    router_handle: UdpRouterHandle,
}

impl UdpListenProcessor {
    pub(crate) async fn start(
        ctx: &Context,
        stream: SplitStream<UdpFramed<TransportMessageCodec>>,
        tx_addr: Address,
        router_handle: UdpRouterHandle,
    ) -> Result<()> {
        let processor = Self {
            stream,
            tx_addr,
            router_handle,
        };
        // FIXME: @ac
        let mailbox = Mailbox::new(
            Address::random_tagged("UdpListenProcessor"),
            Arc::new(AllowAll),
            Arc::new(AllowAll),
        );
        ProcessorBuilder::with_mailboxes(Mailboxes::new(mailbox, vec![]), processor)
            .start(ctx)
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
                Err(_e) => {
                    info!("Failed to read message from UDP socket.");
                    return Ok(false);
                }
            },
            None => {
                info!("No message read, keep socket alive.");
                return Ok(true);
            }
        };

        // Register peer addr with sender half
        // TODO: should `register` be called for every TransportMessage received?
        self.router_handle
            .register(self.tx_addr.clone(), addr.to_string())
            .await?;

        msg.return_route.modify().prepend(UdpAddress::from(addr));

        debug!("Message onward route: {}", msg.onward_route);
        debug!("Message return route: {}", msg.return_route);

        ctx.forward(LocalMessage::new(msg, vec![])).await?;

        Ok(true)
    }
}
