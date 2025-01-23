use crate::transport_message::TcpTransportMessage;
use crate::workers::Addresses;
use crate::{
    TcpConnectionMode, TcpProtocolVersion, TcpReceiverInfo, TcpRegistry, TcpSendWorkerMsg,
    MAX_MESSAGE_SIZE,
};
use core::fmt::Display;
use ockam_core::compat::net::SocketAddr;
use ockam_core::compat::sync::Arc;
use ockam_core::flow_control::FlowControlId;
use ockam_core::{
    async_trait, AllowOnwardAddress, DenyAll, LocalMessage, Mailbox, Mailboxes,
    OutgoingAccessControl,
};
use ockam_core::{Processor, Result};
use ockam_node::{Context, ProcessorBuilder, WorkerShutdownPriority};
use tokio::{io::AsyncReadExt, net::tcp::OwnedReadHalf};
use tracing::{debug, instrument, trace};

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
    incoming_buffer: Vec<u8>,
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
            incoming_buffer: Vec::new(),
            read_half,
            socket_address,
            addresses,
            mode,
            flow_control_id,
        }
    }

    #[allow(clippy::too_many_arguments)]
    #[instrument(skip_all, name = "TcpRecvProcessor::start")]
    pub fn start(
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
            None,
            Arc::new(DenyAll),
            receiver_outgoing_access_control,
        );
        let internal = Mailbox::new(
            addresses.receiver_internal_address().clone(),
            None,
            Arc::new(DenyAll),
            Arc::new(AllowOnwardAddress(
                addresses.sender_internal_address().clone(),
            )),
        );
        ProcessorBuilder::new(receiver)
            .with_mailboxes(Mailboxes::new(mailbox, vec![internal]))
            .with_shutdown_priority(WorkerShutdownPriority::Priority1)
            .start(ctx)?;

        Ok(())
    }

    async fn notify_sender_stream_dropped(&self, ctx: &Context, msg: impl Display) -> Result<()> {
        debug!(
            "Connection to peer '{}' was closed; dropping stream. {}",
            self.socket_address, msg
        );

        ctx.send_from_address(
            self.addresses.sender_internal_address().clone(),
            TcpSendWorkerMsg::ConnectionClosed,
            self.addresses.receiver_internal_address().clone(),
        )
        .await
    }
}

#[async_trait]
impl Processor for TcpRecvProcessor {
    type Context = Context;

    #[instrument(skip_all, name = "TcpRecvProcessor::initialize")]
    async fn initialize(&mut self, ctx: &mut Context) -> Result<()> {
        self.registry.add_receiver_processor(TcpReceiverInfo::new(
            ctx.primary_address().clone(),
            self.addresses.sender_address().clone(),
            self.socket_address,
            self.mode,
            self.flow_control_id.clone(),
        ));

        let protocol_version = match self.read_half.read_u8().await {
            Ok(p) => p,
            Err(e) => {
                trace!("Cannot read the Ockam protocol version: {:?}", &e);
                self.notify_sender_stream_dropped(ctx, e).await?;
                // stop this processor
                ctx.stop_primary_address()?;
                return Ok(());
            }
        };

        let _protocol_version = match TcpProtocolVersion::try_from(protocol_version) {
            Ok(v) => v,
            Err(e) => {
                let message =
                    format!("Received protocol message is unsupported: {protocol_version}");
                trace!("{}: {:?}", &message, &e);
                self.notify_sender_stream_dropped(ctx, message).await?;
                // stop this processor
                ctx.stop_primary_address()?;
                return Ok(());
            }
        };

        Ok(())
    }

    #[instrument(skip_all, name = "TcpRecvProcessor::shutdown")]
    async fn shutdown(&mut self, ctx: &mut Self::Context) -> Result<()> {
        self.registry
            .remove_receiver_processor(ctx.primary_address());

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
    #[instrument(skip_all, name = "TcpRecvProcessor::process", fields(worker = %ctx.primary_address()))]
    async fn process(&mut self, ctx: &mut Context) -> Result<bool> {
        // Read the message length
        let len = match self.read_half.read_u32().await {
            Ok(l) => l,
            Err(e) => {
                self.notify_sender_stream_dropped(ctx, e).await?;
                return Ok(false);
            }
        };

        let len_usize = match usize::try_from(len) {
            Ok(l) => l,
            Err(_) => {
                self.notify_sender_stream_dropped(
                    ctx,
                    format!("Received message len doesn't fit usize: {}", len),
                )
                .await?;
                return Ok(false);
            }
        };

        if len_usize > MAX_MESSAGE_SIZE {
            self.notify_sender_stream_dropped(
                ctx,
                format!(
                    "Received message is larger than allow: {} > {}",
                    len_usize, MAX_MESSAGE_SIZE
                ),
            )
            .await?;
            return Ok(false);
        }

        trace!("Received message header for {} bytes", len);

        // Allocate a buffer of that size
        self.incoming_buffer.clear();
        self.incoming_buffer.reserve(len_usize);
        self.incoming_buffer.resize(len_usize, 0);

        // Then read into the buffer
        match self.read_half.read_exact(&mut self.incoming_buffer).await {
            Ok(_) => {}
            Err(e) => {
                self.notify_sender_stream_dropped(ctx, e).await?;
                return Ok(false);
            }
        }

        // Deserialize the message now
        let transport_message: TcpTransportMessage = match minicbor::decode(&self.incoming_buffer) {
            Ok(msg) => msg,
            Err(e) => {
                self.notify_sender_stream_dropped(ctx, e).await?;
                return Ok(false);
            }
        };

        let local_message = LocalMessage::from(transport_message);
        if !local_message.has_next_on_onward_route() {
            trace!("Got heartbeat message from: {}", self.socket_address);
            return Ok(true);
        }

        // Insert the peer address into the return route so that
        // reply routing can be properly resolved
        let local_message =
            local_message.push_front_return_route(self.addresses.sender_address().clone());

        trace!("Message onward route: {}", local_message.onward_route());
        trace!("Message return route: {}", local_message.return_route());

        // Forward the message to the next hop in the route
        ctx.forward_from_address(local_message, self.addresses.receiver_address().clone())
            .await?;

        Ok(true)
    }
}
