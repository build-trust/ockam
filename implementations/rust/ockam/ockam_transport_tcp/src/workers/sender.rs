use crate::workers::Addresses;
use crate::{TcpConnectionMode, TcpRegistry, TcpSenderInfo};
use ockam_core::flow_control::FlowControlId;
use ockam_core::{
    async_trait,
    compat::{net::SocketAddr, sync::Arc},
    AllowSourceAddress, DenyAll, IncomingAccessControl,
};
use ockam_core::{Any, Decodable, Mailbox, Mailboxes, Message, Result, Routed, Worker};
use ockam_node::{Context, WorkerBuilder};

use ockam_transport_core::encode_transport_message;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;
use tokio::net::tcp::OwnedWriteHalf;
use tracing::{info, instrument, trace, warn};

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
        sender_incoming_access_control: Arc<dyn IncomingAccessControl>,
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
            sender_incoming_access_control,
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
            // Create a message buffer with prepended length
            let transport_message = local_message.into_transport_message();
            let msg = encode_transport_message(transport_message)?;

            if self.write_half.write_all(msg.as_slice()).await.is_err() {
                warn!("Failed to send message to peer {}", self.socket_address);
                self.stop(ctx).await?;

                return Ok(());
            }
        }

        Ok(())
    }
}
