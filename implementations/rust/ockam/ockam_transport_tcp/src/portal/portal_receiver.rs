use crate::PortalInternalMessage;
use core::time::Duration;
use ockam_core::async_trait;
use ockam_core::compat::vec::Vec;
use ockam_core::{route, Address, Processor, Result};
use ockam_node::Context;
use tokio::{io::AsyncReadExt, net::tcp::OwnedReadHalf};
use tracing::{error, warn};

const MAX_PAYLOAD_SIZE: usize = 10 * 1024;

/// A TCP Portal receiving message processor
///
/// TCP Portal receiving message processor are created by
/// `TcpPortalWorker` after a call is made to
/// [`TcpPortalWorker::start_receiver`](crate::TcpPortalWorker::start_receiver)
pub(crate) struct TcpPortalRecvProcessor {
    buf: Vec<u8>,
    rx: OwnedReadHalf,
    sender_address: Address,
}

impl TcpPortalRecvProcessor {
    /// Create a new `TcpPortalRecvProcessor`
    pub fn new(rx: OwnedReadHalf, sender_address: Address) -> Self {
        Self {
            buf: Vec::with_capacity(MAX_PAYLOAD_SIZE),
            rx,
            sender_address,
        }
    }
}

#[async_trait]
impl Processor for TcpPortalRecvProcessor {
    type Context = Context;

    async fn process(&mut self, ctx: &mut Context) -> Result<bool> {
        self.buf.clear();

        let _len = match self.rx.read_buf(&mut self.buf).await {
            Ok(len) => len,
            Err(err) => {
                error!("Tcp Portal connection read failed with error: {}", err);
                return Ok(false);
            }
        };

        if self.buf.is_empty() {
            // Notify Sender that connection was closed
            match ctx
                .send(
                    route![self.sender_address.clone()],
                    PortalInternalMessage::Disconnect,
                )
                .await
            {
                Err(err) => warn!(
                    "Error notifying Tcp Portal Sender about dropped connection {}",
                    err
                ),
                _ => {}
            }

            return Ok(false);
        }

        // Loop just in case buf was extended (should not happen though)
        for chunk in self.buf.chunks(MAX_PAYLOAD_SIZE) {
            let msg = PortalInternalMessage::Payload(chunk.to_vec());

            // Let Sender forward payload to the other side
            ctx.send(route![self.sender_address.clone()], msg).await?;
            ctx.sleep(Duration::from_millis(10)).await;
        }

        Ok(true)
    }
}
