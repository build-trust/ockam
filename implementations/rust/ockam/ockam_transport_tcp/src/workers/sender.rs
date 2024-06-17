use crate::workers::Addresses;
use crate::{TcpConnectionMode, TcpProtocolVersion, TcpRegistry, TcpSenderInfo};
use ockam_core::flow_control::FlowControlId;
use ockam_core::{
    async_trait,
    compat::{net::SocketAddr, sync::Arc},
    AllowAll, AllowSourceAddress, DenyAll, LocalMessage,
};
use ockam_core::{Any, Decodable, Mailbox, Mailboxes, Message, Result, Routed, Worker};
use ockam_node::{Context, WorkerBuilder};

use crate::transport_message::TcpTransportMessage;
use ockam_transport_core::TransportError;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;
use tokio::net::tcp::OwnedWriteHalf;
use tracing::{info, instrument, trace, warn};

/// 16 MB
pub const MAX_MESSAGE_SIZE: usize = 16 * 1024 * 1024;

#[derive(Serialize, Deserialize, Message, Clone)]
pub(crate) enum TcpSendWorkerMsg {
    ConnectionClosed,
}

/// A TCP sending message worker
///
/// Create this worker type by calling
/// [`TcpSendWorker::start_pair`](crate::TcpSendWorker::start_pair)
///
/// This half of the worker is created when spawning a new connection
/// worker pair, and listens for messages from the node message system
/// to dispatch to a remote peer.
pub(crate) struct TcpSendWorker {
    registry: TcpRegistry,
    write_half: OwnedWriteHalf,
    socket_address: SocketAddr,
    addresses: Addresses,
    mode: TcpConnectionMode,
    receiver_flow_control_id: FlowControlId,
    rx_should_be_stopped: bool,
}

impl TcpSendWorker {
    /// Create a new `TcpSendWorker`
    fn new(
        registry: TcpRegistry,
        write_half: OwnedWriteHalf,
        socket_address: SocketAddr,
        addresses: Addresses,
        mode: TcpConnectionMode,
        receiver_flow_control_id: FlowControlId,
    ) -> Self {
        Self {
            registry,
            write_half,
            socket_address,
            addresses,
            receiver_flow_control_id,
            mode,
            rx_should_be_stopped: true,
        }
    }
}

impl TcpSendWorker {
    /// Create a `(TcpSendWorker, TcpRecvProcessor)` pair that opens and
    /// manages the connection with the given peer
    #[allow(clippy::too_many_arguments)]
    #[instrument(skip_all, name = "TcpSendWorker::start")]
    pub(crate) async fn start(
        ctx: &Context,
        registry: TcpRegistry,
        write_half: OwnedWriteHalf,
        addresses: &Addresses,
        socket_address: SocketAddr,
        mode: TcpConnectionMode,
        receiver_flow_control_id: &FlowControlId,
    ) -> Result<()> {
        trace!("Creating new TCP worker pair");
        let sender_worker = Self::new(
            registry,
            write_half,
            socket_address,
            addresses.clone(),
            mode,
            receiver_flow_control_id.clone(),
        );

        let main_mailbox = Mailbox::new(
            addresses.sender_address().clone(),
            Arc::new(AllowAll),
            Arc::new(DenyAll),
        );

        let internal_mailbox = Mailbox::new(
            addresses.sender_internal_address().clone(),
            Arc::new(AllowSourceAddress(
                addresses.receiver_internal_address().clone(),
            )),
            Arc::new(DenyAll),
        );

        WorkerBuilder::new(sender_worker)
            .with_mailboxes(Mailboxes::new(main_mailbox.clone(), vec![internal_mailbox]))
            .terminal(addresses.sender_address().clone())
            .start(ctx)
            .await?;

        Ok(())
    }

    #[instrument(skip_all, name = "TcpSendWorker::stop")]
    async fn stop(&self, ctx: &Context) -> Result<()> {
        ctx.stop_worker(self.addresses.sender_address().clone())
            .await?;

        Ok(())
    }

    fn serialize_message(&self, local_message: LocalMessage) -> Result<Vec<u8>> {
        // Create a message buffer with prepended length
        let transport_message = TcpTransportMessage::from(local_message);

        let msg_len = minicbor::len(&transport_message);

        if msg_len > MAX_MESSAGE_SIZE {
            return Err(TransportError::MessageLengthExceeded)?;
        }

        // Prepending message with u32 (4 bytes) length
        let len = 4 + msg_len;

        let msg_len_u32 =
            u32::try_from(msg_len).map_err(|_| TransportError::MessageLengthExceeded)?;

        let mut vec = vec![0u8; len];

        vec[..4].copy_from_slice(&msg_len_u32.to_be_bytes());
        minicbor::encode(&transport_message, &mut vec[4..])
            .map_err(|_| TransportError::Encoding)?;

        Ok(vec)
    }
}

#[async_trait]
impl Worker for TcpSendWorker {
    type Context = Context;
    type Message = Any;

    #[instrument(skip_all, name = "TcpSendWorker::initialize")]
    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        ctx.set_cluster(crate::CLUSTER_NAME).await?;

        self.registry.add_sender_worker(TcpSenderInfo::new(
            self.addresses.sender_address().clone(),
            self.addresses.receiver_address().clone(),
            self.socket_address,
            self.mode,
            self.receiver_flow_control_id.clone(),
        ));

        // First thing send our protocol version
        if self
            .write_half
            .write_u8(TcpProtocolVersion::V1.into())
            .await
            .is_err()
        {
            warn!(
                "Failed to send protocol version to peer {}",
                self.socket_address
            );
            self.stop(ctx).await?;

            return Ok(());
        }

        Ok(())
    }

    #[instrument(skip_all, name = "TcpSendWorker::shutdown")]
    async fn shutdown(&mut self, ctx: &mut Self::Context) -> Result<()> {
        self.registry
            .remove_sender_worker(self.addresses.sender_address());

        if self.rx_should_be_stopped {
            let _ = ctx
                .stop_processor(self.addresses.receiver_address().clone())
                .await;
        }

        Ok(())
    }

    // TcpSendWorker will receive messages from the TcpRouter to send
    // across the TcpStream to our friend
    #[instrument(skip_all, name = "TcpSendWorker::handle_message", fields(worker = %ctx.address()))]
    async fn handle_message(
        &mut self,
        ctx: &mut Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        let recipient = msg.msg_addr();
        if &recipient == self.addresses.sender_internal_address() {
            let msg = TcpSendWorkerMsg::decode(msg.payload())?;

            match msg {
                TcpSendWorkerMsg::ConnectionClosed => {
                    info!(
                        "Stopping sender due to closed connection {}",
                        self.socket_address
                    );
                    // No need to stop Receiver as it notified us about connection drop and will
                    // stop itself
                    self.rx_should_be_stopped = false;
                    self.stop(ctx).await?;

                    return Ok(());
                }
            }
        } else {
            let mut local_message = msg.into_local_message();
            // Remove our own address from the route so the other end
            // knows what to do with the incoming message
            local_message = local_message.pop_front_onward_route()?;

            let msg = match self.serialize_message(local_message) {
                Ok(msg) => msg,
                Err(err) => {
                    // Close the stream
                    self.stop(ctx).await?;

                    return Err(err);
                }
            };

            if self.write_half.write_all(&msg).await.is_err() {
                warn!("Failed to send message to peer {}", self.socket_address);
                self.stop(ctx).await?;

                return Ok(());
            }
        }

        Ok(())
    }
}
