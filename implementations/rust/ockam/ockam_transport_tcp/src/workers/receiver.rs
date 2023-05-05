use crate::workers::Addresses;
use crate::{TcpConnectionMode, TcpReceiverInfo, TcpRegistry, TcpSendWorkerMsg};
use ockam_core::compat::net::SocketAddr;
use ockam_core::compat::sync::Arc;
use ockam_core::flow_control::FlowControlId;
use ockam_core::{
    async_trait, AllowOnwardAddress, DenyAll, Mailbox, Mailboxes, OutgoingAccessControl,
};
use ockam_core::{Decodable, LocalMessage, Processor, Result, TransportMessage};
use ockam_node::{Context, ProcessorBuilder};
use ockam_transport_core::TransportError;
use tokio::{io::AsyncReadExt, net::tcp::OwnedReadHalf};
use tracing::{error, info, trace};

/// A TCP receiving message processor
///
/// Create this processor type by calling
/// [`TcpSendWorker::start_pair`](crate::TcpSendWorker::start_pair)
///
/// This half of the worker is created when spawning a new connection
/// worker pair, and listens for incoming TCP packets, to relay into
/// the node message system.
pub(crate) struct TcpRecvProcessor {
    registry: TcpRegistry,
    read_half: OwnedReadHalf,
    socket_address: SocketAddr,
    addresses: Addresses,
    mode: TcpConnectionMode,
    flow_control_id: FlowControlId,
}

impl TcpRecvProcessor {
    /// Create a new `TcpRecvProcessor`
    fn new(
        registry: TcpRegistry,
        read_half: OwnedReadHalf,
        socket_address: SocketAddr,
        addresses: Addresses,
        mode: TcpConnectionMode,
        flow_control_id: FlowControlId,
    ) -> Self {
        Self {
            registry,
            read_half,
            socket_address,
            addresses,
            mode,
            flow_control_id,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn start(
        ctx: &Context,
        registry: TcpRegistry,
        read_half: OwnedReadHalf,
        addresses: &Addresses,
        socket_address: SocketAddr,
        mode: TcpConnectionMode,
        flow_control_id: &FlowControlId,
        receiver_outgoing_access_control: Arc<dyn OutgoingAccessControl>,
    ) -> Result<()> {
        let receiver = TcpRecvProcessor::new(
            registry,
            read_half,
            socket_address,
            addresses.clone(),
            mode,
            flow_control_id.clone(),
        );

        let mailbox = Mailbox::new(
            addresses.receiver_address().clone(),
            Arc::new(DenyAll),
            receiver_outgoing_access_control,
        );
        let internal = Mailbox::new(
            addresses.receiver_internal_address().clone(),
            Arc::new(DenyAll),
            Arc::new(AllowOnwardAddress(
                addresses.sender_internal_address().clone(),
            )),
        );
        ProcessorBuilder::with_mailboxes(Mailboxes::new(mailbox, vec![internal]), receiver)
            .start(ctx)
            .await?;

        Ok(())
    }
}

#[async_trait]
impl Processor for TcpRecvProcessor {
    type Context = Context;

    async fn initialize(&mut self, ctx: &mut Context) -> Result<()> {
        ctx.set_cluster(crate::CLUSTER_NAME).await?;

        self.registry.add_receiver_processor(TcpReceiverInfo::new(
            ctx.address(),
            self.addresses.sender_address().clone(),
            self.socket_address,
            self.mode,
            self.flow_control_id.clone(),
        ));

        Ok(())
    }

    async fn shutdown(&mut self, ctx: &mut Self::Context) -> Result<()> {
        self.registry.remove_receiver_processor(&ctx.address());

        Ok(())
    }

    /// Get the next message from the connection if there are any
    /// available and forward it to the next hop in the route.
    ///
    /// Notes:
    ///
    /// 1. We are using the initialize function here to run a custom loop,
    ///    instead of listening for messages sent to our address.
    /// 2. When the loop exits, we _must_ call stop_worker(..) on
    ///    Context to avoid spawning a zombie task.
    /// 3. We must also stop the TcpReceive loop when the worker gets
    ///    killed by the user or node.
    async fn process(&mut self, ctx: &mut Context) -> Result<bool> {
        // Run in a loop until TcpWorkerPair::stop() is called
        // First read a message length header...
        let len = match self.read_half.read_u16().await {
            Ok(len) => len,
            Err(_e) => {
                info!(
                    "Connection to peer '{}' was closed; dropping stream",
                    self.socket_address
                );

                // Notify sender tx is closed
                ctx.send_from_address(
                    self.addresses.sender_internal_address().clone(),
                    TcpSendWorkerMsg::ConnectionClosed,
                    self.addresses.receiver_internal_address().clone(),
                )
                .await?;

                return Ok(false);
            }
        };

        trace!("Received message header for {} bytes", len);

        // Allocate a buffer of that size
        let mut buf = vec![0; len as usize];

        // Then read into the buffer
        match self.read_half.read_exact(&mut buf).await {
            Ok(_) => {}
            _ => {
                error!("Failed to receive message of length: {}", len);
                return Ok(true);
            }
        }

        // Deserialize the message now
        let mut msg = TransportMessage::decode(&buf).map_err(|_| TransportError::RecvBadMessage)?;

        // Heartbeat message
        if msg.onward_route.next().is_err() {
            trace!("Got heartbeat message from: {}", self.socket_address);
            return Ok(true);
        }

        // Insert the peer address into the return route so that
        // reply routing can be properly resolved
        msg.return_route
            .modify()
            .prepend(self.addresses.sender_address().clone());

        trace!("Message onward route: {}", msg.onward_route);
        trace!("Message return route: {}", msg.return_route);

        // Forward the message to the next hop in the route
        ctx.forward_from_address(
            LocalMessage::new(msg, vec![]),
            self.addresses.receiver_address().clone(),
        )
        .await?;

        Ok(true)
    }
}
