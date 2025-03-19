use crate::portal::addresses::{Addresses, PortalType};
use crate::{PortalInternalMessage, PortalMessage, TcpRegistry};
use ockam_core::compat::vec::Vec;
use ockam_core::{async_trait, Encodable, LocalMessage, Route, OCKAM_TRACER_NAME};
use ockam_core::{route, Processor, Result};
use ockam_node::Context;
use opentelemetry::trace::{Span, Tracer};
use opentelemetry::{global, KeyValue};
use tokio::io::AsyncRead;
use tokio::io::AsyncReadExt;
use tracing::{debug, error, instrument, Level};

/// A TCP Portal receiving message processor
///
/// TCP Portal receiving message processor are created by
/// `TcpPortalWorker` after a call is made to
/// [`TcpPortalWorker::start_receiver`](crate::TcpPortalWorker::start_receiver)
pub(crate) struct TcpPortalRecvProcessor<R> {
    registry: TcpRegistry,
    buf: Vec<u8>,
    read_half: R,
    addresses: Addresses,
    onward_route: Route,
    payload_packet_counter: u16,
    portal_payload_length: usize,
}

impl<R: AsyncRead + Unpin + Send + Sync + 'static> TcpPortalRecvProcessor<R> {
    /// Create a new `TcpPortalRecvProcessor`
    pub fn new(
        registry: TcpRegistry,
        read_half: R,
        addresses: Addresses,
        onward_route: Route,
        portal_payload_length: usize,
    ) -> Self {
        Self {
            registry,
            buf: Vec::with_capacity(portal_payload_length),
            read_half,
            addresses,
            onward_route,
            payload_packet_counter: 0,
            portal_payload_length,
        }
    }
}

#[async_trait]
impl<R: AsyncRead + Unpin + Send + Sync + 'static> Processor for TcpPortalRecvProcessor<R> {
    type Context = Context;

    #[instrument(skip_all, name = "TcpPortalRecvProcessor::initialize", level = Level::TRACE)]
    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        self.registry
            .add_portal_receiver_processor(ctx.primary_address());

        Ok(())
    }

    #[instrument(skip_all, name = "TcpPortalRecvProcessor::shutdown", level = Level::TRACE)]
    async fn shutdown(&mut self, ctx: &mut Self::Context) -> Result<()> {
        self.registry
            .remove_portal_receiver_processor(ctx.primary_address());

        Ok(())
    }

    async fn process(&mut self, ctx: &mut Context) -> Result<bool> {
        self.buf.clear();

        let _len = match self.read_half.read_buf(&mut self.buf).await {
            Ok(len) => len,
            Err(err) => {
                error!("Tcp Portal connection read failed with error: {}", err);
                return Ok(false);
            }
        };

        if self.buf.is_empty() {
            // Notify Sender that connection was closed
            if let Err(err) = ctx
                .send_from_address(
                    route![self.addresses.sender_internal.clone()],
                    PortalInternalMessage::Disconnect,
                    self.addresses.receiver_internal.clone(),
                )
                .await
            {
                debug!(
                    "Error notifying Tcp Portal Sender about dropped connection {}",
                    err
                );
            }
            return self.disconnect(ctx).await;
        };

        self.send_message(ctx).await
    }
}

impl<R: AsyncRead + Unpin + Send + Sync + 'static> TcpPortalRecvProcessor<R> {
    async fn send_message(&mut self, ctx: &mut Context) -> Result<bool> {
        let span = self.start_span(ctx)?;
        ctx.set_tracing_context_from_span(span);

        // Loop just in case buf was extended (should not happen though)
        for chunk in self.buf.chunks(self.portal_payload_length) {
            let msg = LocalMessage::new()
                .with_tracing_context(ctx.tracing_context())
                .with_onward_route(self.onward_route.clone())
                .with_return_route(route![self.addresses.sender_remote.clone()])
                .with_payload(
                    PortalMessage::Payload(chunk, Some(self.payload_packet_counter)).encode()?,
                );

            self.payload_packet_counter += 1;
            ctx.forward_from_address(msg, self.addresses.receiver_remote.clone())
                .await?;
        }

        Ok(true)
    }

    async fn disconnect(&mut self, ctx: &mut Context) -> Result<bool> {
        if let Err(err) = ctx
            .forward_from_address(
                LocalMessage::new()
                    .with_onward_route(self.onward_route.clone())
                    .with_return_route(route![self.addresses.sender_remote.clone()])
                    .with_payload(PortalMessage::Disconnect.encode()?),
                self.addresses.receiver_remote.clone(),
            )
            .await
        {
            debug!(
                "Error notifying the other side of the portal about dropped connection {}",
                err
            );
        }

        Ok(false)
    }

    fn start_span(&self, ctx: &Context) -> Result<impl Span> {
        let name = match self.addresses.portal_type {
            PortalType::Inlet { .. } => "receive_tcp_message_at_inlet",
            PortalType::Outlet => "receive_tcp_message_at_outlet",
            PortalType::PrivilegedInlet { .. } => "receive_tcp_message_at_privileged_inlet",
            PortalType::PrivilegedOutlet => "receive_tcp_message_at_privileged_outlet",
        };
        let tracer = global::tracer(OCKAM_TRACER_NAME);
        let span = tracer
            .span_builder(name)
            .with_attributes(vec![
                KeyValue::new("portal_type", self.addresses.portal_type.to_string()),
                KeyValue::new("onward_route", self.onward_route.to_string()),
                KeyValue::new("worker_address", ctx.primary_address().to_string()),
                KeyValue::new(
                    "worker_other_addresses",
                    ctx.additional_addresses()
                        .map(|a| a.to_string())
                        .collect::<Vec<String>>()
                        .join(",")
                        .to_string(),
                ),
            ])
            .start(&tracer);
        Ok(span)
    }
}
