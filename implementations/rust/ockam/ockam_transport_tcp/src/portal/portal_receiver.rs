use crate::PortalInternalMessage;
use ockam_core::async_trait;
use ockam_core::{route, Address, Processor, Result};
use ockam_node::Context;
use tokio::{io::AsyncReadExt, net::tcp::OwnedReadHalf};
use tracing::error;

const BUFFER_SIZE: usize = 256;

/// A TCP Portal receiving message processor
///
/// TCP Portal receiving message processor are created by
/// `TcpPortalWorker` after a call is made to
/// [`TcpPortalWorker::start_receiver`](crate::TcpPortalWorker::start_receiver)
pub(crate) struct TcpPortalRecvProcessor {
    rx: OwnedReadHalf,
    sender_address: Address,
}

impl TcpPortalRecvProcessor {
    /// Create a new `TcpPortalRecvProcessor`
    pub fn new(rx: OwnedReadHalf, sender_address: Address) -> Self {
        Self { rx, sender_address }
    }
}

#[async_trait]
impl Processor for TcpPortalRecvProcessor {
    type Context = Context;

    async fn process(&mut self, ctx: &mut Context) -> Result<bool> {
        let mut buf = [0u8; BUFFER_SIZE];
        let len = match self.rx.read(&mut buf).await {
            Ok(len) => len,
            Err(err) => {
                error!("Tcp Portal connection read failed with error: {}", err);
                return Ok(false);
            }
        };

        if len != 0 {
            let mut vec = vec![0u8; len];
            vec.copy_from_slice(&buf[..len]);
            let msg = PortalInternalMessage::Payload(vec);

            // Let Sender forward payload to the other side
            ctx.send(route![self.sender_address.clone()], msg).await?;

            Ok(true)
        } else {
            // Notify Sender that connection was closed
            ctx.send(
                route![self.sender_address.clone()],
                PortalInternalMessage::Disconnect,
            )
            .await?;

            Ok(false)
        }
    }
}
