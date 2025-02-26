use core::fmt;
use futures_core::future::BoxFuture;
use opentelemetry::trace::TraceError;
use opentelemetry_proto::tonic::collector::trace::v1::{
    trace_service_client::TraceServiceClient, ExportTraceServiceRequest,
};
use opentelemetry_sdk::export::trace::{ExportResult, SpanData, SpanExporter};
use tonic::{codegen::CompressionEncoding, Request};

use crate::logs::secure_client_service::SecureClientService;
use opentelemetry_proto::transform::trace::tonic::group_spans_by_resource_and_scope;
use tonic::metadata::{KeyAndValueRef, MetadataMap};

/// This struct does what most of the TonicTracesClient does as a SpanExporter, except that
/// it uses a SecureClientService to send the gRPC requests serialized as http Request to
/// a gRPC forwarder service located in another Ockam node, via a secure channel.
///
/// Note that the original TonicTracesClient can also be parameterized with an Interceptor to:
///  - Potentially drop some requests
///  - Add some metadata or extensions to the request
///
/// We don't use those capabilities so there is no need to use an Interceptor here.
pub struct OckamTonicTracesClient {
    inner: Option<ClientInner>,
    additional_headers: MetadataMap,
    #[allow(dead_code)]
    // <allow dead> would be removed once we support set_resource for metrics.
    resource: opentelemetry_proto::transform::common::tonic::ResourceAttributesWithSchema,
}

#[derive(Clone)]
struct ClientInner {
    client: TraceServiceClient<SecureClientService>,
}

impl fmt::Debug for OckamTonicTracesClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("OckamTonicTracesClient")
    }
}

impl OckamTonicTracesClient {
    /// Create a new OckamTonicTracesClient with the given SecureClientService and potentially
    /// some compression via gzip or zstd.
    pub fn new(
        secure_client_service: SecureClientService,
        additional_headers: MetadataMap,
        compression: Option<CompressionEncoding>,
    ) -> Self {
        let mut client = TraceServiceClient::new(secure_client_service);
        if let Some(compression) = compression {
            client = client
                .send_compressed(compression)
                .accept_compressed(compression);
        }

        OckamTonicTracesClient {
            inner: Some(ClientInner { client }),
            additional_headers,
            resource: Default::default(),
        }
    }
}

/// Implement the SpanExporter trait for OckamTonicTracesClient
/// If the inner client is available, use it to export the traces as an ExportTraceServiceRequest
/// to the remote collector, via the secure channel and a gRPC forwarder service on the other node.
impl SpanExporter for OckamTonicTracesClient {
    fn export(&mut self, batch: Vec<SpanData>) -> BoxFuture<'static, ExportResult> {
        if let Some(ref mut client) = &mut self.inner {
            let resource_spans = group_spans_by_resource_and_scope(batch, &self.resource);
            let mut request = Request::new(ExportTraceServiceRequest { resource_spans });
            for key_and_value in self.additional_headers.iter() {
                match key_and_value {
                    KeyAndValueRef::Ascii(key, value) => {
                        request.metadata_mut().append(key, value.to_owned())
                    }
                    KeyAndValueRef::Binary(key, value) => {
                        request.metadata_mut().append_bin(key, value.to_owned())
                    }
                };
            }

            let mut client_clone = client.clone();
            Box::pin(async move {
                client_clone
                    .client
                    .export(request)
                    .await
                    .map_err(opentelemetry_otlp::Error::from)?;

                Ok(())
            })
        } else {
            Box::pin(std::future::ready(Err(TraceError::Other(
                "The span exporter is already shut down".into(),
            ))))
        }
    }

    fn shutdown(&mut self) {
        let _ = self.inner.take();
    }

    fn set_resource(&mut self, resource: &opentelemetry_sdk::Resource) {
        self.resource = resource.into();
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::authority_node::tests::random_port;
    use crate::cli_state::{CliStateMode, UseAwsKms};
    use crate::logs::grpc_forwarder::GrpcForwarder;
    use crate::{ApiError, CliState, DefaultAddress, Result};
    use hyper::Uri;
    use ockam::identity::{
        SecureChannelListener, SecureChannelListenerOptions, SecureChannels, SecureClient,
        TrustEveryonePolicy,
    };
    use ockam_core::{route, Address};
    use ockam_node::{Context, NodeBuilder};
    use ockam_transport_tcp::{TcpListenerOptions, TcpTransport, TCP};
    use opentelemetry::trace::{SpanContext, SpanId, SpanKind};
    use std::sync::Arc;
    use std::time::{Duration, SystemTime};
    use tokio::runtime::Runtime;

    const LOGGING: bool = true;

    #[ignore]
    #[test]
    fn test_export_spans() {
        let runtime = Arc::new(Runtime::new().unwrap());
        let port = random_port();
        start_node_with_grpc_forwarder_service(runtime.clone(), port);

        let (ctx, mut executor) = NodeBuilder::new()
            .with_logging(LOGGING)
            .with_runtime(runtime)
            .build();
        let result = executor.execute::<_, (), ApiError>(async move {
            // wait until the forwarder node is up
            tokio::time::sleep(Duration::from_millis(100)).await;
            let secure_channels = create_secure_channels().await?;
            let tcp_transport = TcpTransport::get_or_create(&ctx)?;
            let secure_client = make_secure_client(port, secure_channels, tcp_transport).await?;
            let project_service =
                SecureClientService::new(secure_client, &ctx, DefaultAddress::GRPC_FORWARDER);
            let mut exporter =
                OckamTonicTracesClient::new(project_service, Default::default(), None);
            exporter
                .export(create_spans())
                .await
                .map_err(ApiError::message)?;
            ctx.shutdown_node().await?;
            Ok(())
        });

        assert!(result.is_ok());
    }

    // HELPERS

    /// Create a SecureChannels service for a local node
    pub(crate) async fn create_secure_channels() -> Result<Arc<SecureChannels>> {
        let cli_state = CliState::create(CliStateMode::InMemory).await?;
        cli_state.create_node("default").await?;
        cli_state
            .create_named_vault(None, None, UseAwsKms::No)
            .await?;
        Ok(cli_state.secure_channels_for_node("default").await?)
    }

    /// Create an arbitrary batch of spans
    fn create_spans() -> Vec<SpanData> {
        let span = SpanData {
            span_context: SpanContext::empty_context(),
            parent_span_id: SpanId::from(1),
            span_kind: SpanKind::Client,
            name: Default::default(),
            start_time: SystemTime::now(),
            end_time: SystemTime::now(),
            attributes: vec![],
            dropped_attributes_count: 0,
            events: Default::default(),
            links: Default::default(),
            status: Default::default(),
            instrumentation_scope: Default::default(),
        };
        vec![span]
    }

    /// Create a secure client to a local node having a TCP listener on the given port
    /// For the purpose of this test, the node is assumed to accept any identifier connecting to it.
    pub(crate) async fn make_secure_client(
        port: u16,
        secure_channels: Arc<SecureChannels>,
        tcp_transport: Arc<TcpTransport>,
    ) -> Result<SecureClient> {
        let route = route![(TCP, format!("127.0.0.1:{port}")), "api"];
        let client_identifier = secure_channels
            .identities()
            .identities_creation()
            .create_identity()
            .await?;
        secure_channels
            .identities()
            .purpose_keys()
            .purpose_keys_creation()
            .get_or_create_credential_purpose_key(&client_identifier)
            .await?;
        Ok(SecureClient::new(
            secure_channels,
            None,
            tcp_transport,
            route,
            Arc::new(TrustEveryonePolicy),
            &client_identifier,
            Duration::MAX,
            Duration::MAX,
        ))
    }

    /// Start a node with a GRPC forwarder service.
    /// That service connects to a local OpenTelemetry collector.
    pub(crate) fn start_node_with_grpc_forwarder_service(runtime: Arc<Runtime>, port: u16) {
        let runtime_clone = runtime.clone();
        runtime.spawn(async move {
            let (ctx, _executor) = NodeBuilder::new()
                .with_logging(LOGGING)
                .with_runtime(runtime_clone)
                .build();

            // start a TCP listener, and a secure channel listener and the grpc forwarder service.
            let tcp_listener_options = start_tcp_listener(&ctx, port).await?;
            let secure_channel_listener =
                start_secure_channel_listener(&ctx, tcp_listener_options).await?;
            start_forwarder_service(&ctx, secure_channel_listener, "http://127.0.0.1:4317").await?;

            let _ = tokio::time::sleep(Duration::MAX).await;
            Ok::<(), ApiError>(())
        });
    }

    pub(crate) async fn start_tcp_listener(ctx: &Context, port: u16) -> Result<TcpListenerOptions> {
        let tcp_transport = TcpTransport::get_or_create(ctx)?;
        let tcp_listener_options = TcpListenerOptions::new();
        let _tcp_listener = tcp_transport
            .listen(format!("127.0.0.1:{port}"), tcp_listener_options.clone())
            .await?;
        Ok(tcp_listener_options)
    }

    async fn start_secure_channel_listener(
        ctx: &Context,
        tcp_listener_options: TcpListenerOptions,
    ) -> Result<SecureChannelListener> {
        let secure_channels = create_secure_channels().await?;
        let identities = secure_channels.identities();

        let telemetry_node_identifier = identities.identities_creation().create_identity().await?;
        identities
            .purpose_keys()
            .purpose_keys_creation()
            .get_or_create_credential_purpose_key(&telemetry_node_identifier)
            .await?;
        let secure_channel_listener_options = SecureChannelListenerOptions::new()
            .as_consumer(&tcp_listener_options.spawner_flow_control_id())
            .with_trust_policy(TrustEveryonePolicy);
        secure_channels
            .create_secure_channel_listener(
                ctx,
                &telemetry_node_identifier,
                "api",
                secure_channel_listener_options,
            )
            .map_err(ApiError::message)
    }

    async fn start_forwarder_service(
        ctx: &Context,
        secure_channel_listener: SecureChannelListener,
        endpoint: &'static str,
    ) -> Result<()> {
        let uri = Uri::from_static(endpoint);
        ctx.start_worker(
            DefaultAddress::GRPC_FORWARDER,
            GrpcForwarder::new(uri).await?,
        )?;
        ctx.flow_controls().add_consumer(
            &Address::from_string(DefaultAddress::GRPC_FORWARDER),
            secure_channel_listener.flow_control_id(),
        );
        Ok(())
    }
}
