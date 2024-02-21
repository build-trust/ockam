use crate::portal::portal_message::MAX_PAYLOAD_SIZE;
use crate::{PortalInternalMessage, PortalMessage, TcpRegistry};
use ockam_core::compat::vec::Vec;
use ockam_core::{
    async_trait, Encodable, LocalMessage, OpenTelemetryContext, Route, OCKAM_TRACER_NAME,
};
use ockam_core::{route, Address, Processor, Result};
use ockam_node::Context;
use opentelemetry::global;
use opentelemetry::trace::Tracer;
use tokio::{io::AsyncReadExt, net::tcp::OwnedReadHalf};
use tracing::{error, instrument, warn};

/// A TCP Portal receiving message processor
///
/// TCP Portal receiving message processor are created by
/// `TcpPortalWorker` after a call is made to
/// [`TcpPortalWorker::start_receiver`](crate::TcpPortalWorker::start_receiver)
pub(crate) struct TcpPortalRecvProcessor {
    registry: TcpRegistry,
    buf: Vec<u8>,
    read_half: OwnedReadHalf,
    sender_address: Address,
    onward_route: Route,
    payload_packet_counter: u16,
}

impl TcpPortalRecvProcessor {
    /// Create a new `TcpPortalRecvProcessor`
    pub fn new(
        registry: TcpRegistry,
        read_half: OwnedReadHalf,
        sender_address: Address,
        onward_route: Route,
    ) -> Self {
        Self {
            registry,
            buf: Vec::with_capacity(MAX_PAYLOAD_SIZE),
            read_half,
            sender_address,
            onward_route,
            payload_packet_counter: 0,
        }
    }
}

#[async_trait]
impl Processor for TcpPortalRecvProcessor {
    type Context = Context;

    #[instrument(skip_all, name = "TcpPortalRecvProcessor::initialize")]
    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        self.registry.add_portal_receiver_processor(&ctx.address());

        Ok(())
    }

    #[instrument(skip_all, name = "TcpPortalRecvProcessor::shutdown")]
    async fn shutdown(&mut self, ctx: &mut Self::Context) -> Result<()> {
        self.registry
            .remove_portal_receiver_processor(&ctx.address());

        Ok(())
    }

    #[instrument(skip_all, name = "TcpPortalRecvProcessor::process")]
    async fn process(&mut self, ctx: &mut Context) -> Result<bool> {
        self.buf.clear();

        let _len = match self.read_half.read_buf(&mut self.buf).await {
            Ok(len) => len,
            Err(err) => {
                error!("Tcp Portal connection read failed with error: {}", err);
                return Ok(false);
            }
        };

        let tracer = global::tracer(OCKAM_TRACER_NAME);
        let tracing_context = tracer.in_span("TcpPortalRecvProcessor::forward_message", |cx| {
            OpenTelemetryContext::inject(&cx)
        });

        if self.buf.is_empty() {
            // Notify Sender that connection was closed
            ctx.set_tracing_context(tracing_context.clone());
            if let Err(err) = ctx
                .send(
                    route![self.sender_address.clone()],
                    PortalInternalMessage::Disconnect,
                )
                .await
            {
                warn!(
                    "Error notifying Tcp Portal Sender about dropped connection {}",
                    err
                );
            }

            ctx.forward(
                LocalMessage::new()
                    .with_tracing_context(tracing_context.clone())
                    .with_onward_route(self.onward_route.clone())
                    .with_return_route(route![self.sender_address.clone()])
                    .with_payload(PortalMessage::Disconnect.encode()?),
            )
            .await?;

            return Ok(false);
        }

        // Loop just in case buf was extended (should not happen though)
        for chunk in self.buf.chunks(MAX_PAYLOAD_SIZE) {
            let msg = LocalMessage::new()
                .with_tracing_context(tracing_context.clone())
                .with_onward_route(self.onward_route.clone())
                .with_return_route(route![self.sender_address.clone()])
                .with_payload(
                    PortalMessage::Payload(chunk, Some(self.payload_packet_counter)).encode()?,
                );

            self.payload_packet_counter += 1;
            ctx.forward(msg).await?;
        }

        Ok(true)
    }
}
