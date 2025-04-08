use crate::logs::secure_client_service::SecureClientService;
use core::fmt;
use opentelemetry_proto::tonic::collector::logs::v1::{
    logs_service_client::LogsServiceClient, ExportLogsServiceRequest,
};
use opentelemetry_proto::transform::logs::tonic::group_logs_by_resource_and_scope;
use opentelemetry_sdk::export::logs::{LogBatch, LogExporter};
use opentelemetry_sdk::logs::{LogError, LogResult};
use tonic::metadata::{KeyAndValueRef, MetadataMap};
use tonic::{async_trait, codegen::CompressionEncoding, Request};

/// This struct does what most of the TonicLogsClient does as a LogExporter, except that
/// it uses a SecureClientService to send the gRPC requests serialized as http Request to
/// a gRPC forwarder service located in another Ockam node, via a secure channel.
///
/// Note that the original TonicLogsClient can also be parameterized with an Interceptor to:
///  - Potentially drop some requests
///  - Add some metadata or extensions to the request
///
/// We don't use those capabilities so there is no need to use an Interceptor here.
pub struct OckamTonicLogsClient {
    inner: Option<ClientInner>,
    additional_headers: MetadataMap,
    #[allow(dead_code)]
    // <allow dead> would be removed once we support set_resource for metrics.
    resource: opentelemetry_proto::transform::common::tonic::ResourceAttributesWithSchema,
}

#[derive(Clone)]
struct ClientInner {
    client: LogsServiceClient<SecureClientService>,
}

impl fmt::Debug for OckamTonicLogsClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("OckamTonicLogsClient")
    }
}

impl OckamTonicLogsClient {
    /// Create a new OckamTonicLogsClient with the given SecureClientService and potentially
    /// some compression via gzip or zstd.
    pub fn new(
        secure_client_service: SecureClientService,
        additional_headers: MetadataMap,
        compression: Option<CompressionEncoding>,
    ) -> Self {
        let mut client = LogsServiceClient::new(secure_client_service);
        if let Some(compression) = compression {
            client = client
                .send_compressed(compression)
                .accept_compressed(compression);
        }

        OckamTonicLogsClient {
            inner: Some(ClientInner { client }),
            additional_headers,
            resource: Default::default(),
        }
    }
}

/// Implement the LogsExporter trait for OckamTonicLogsClient
/// If the inner client is available, use it to export the logs as an ExportLogsServiceRequest
/// to the remote collector, via the secure channel and a gRPC forwarder service on the other node.
#[async_trait]
impl LogExporter for OckamTonicLogsClient {
    async fn export(&mut self, batch: LogBatch<'_>) -> LogResult<()> {
        if let Some(ref mut client) = &mut self.inner {
            let mut client_clone = client.clone();
            let resource_logs = group_logs_by_resource_and_scope(batch, &self.resource);
            let mut request = Request::new(ExportLogsServiceRequest { resource_logs });
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

            client_clone
                .client
                .export(request)
                .await
                .map_err(opentelemetry_otlp::Error::from)?;
            Ok(())
        } else {
            Err(LogError::Other(
                "The log exporter is already shut down".into(),
            ))
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
mod tests {
    use super::*;
    use crate::authority_node::tests::random_port;
    use crate::logs::ockam_tonic_traces_client::tests::*;
    use crate::{ApiError, DefaultAddress};
    use ockam_node::NodeBuilder;
    use ockam_transport_tcp::TcpTransport;
    use opentelemetry::InstrumentationScope;
    use opentelemetry_sdk::logs::LogRecord;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::runtime::Runtime;

    const LOGGING: bool = true;

    #[ignore]
    #[test]
    fn test_export_logs() {
        let runtime = Arc::new(Runtime::new().unwrap());
        let port = random_port();
        start_node_with_grpc_forwarder_service(runtime.clone(), port);

        let (ctx, executor) = NodeBuilder::new()
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
            let mut exporter = OckamTonicLogsClient::new(project_service, Default::default(), None);
            let record = LogRecord::default();
            let scope = InstrumentationScope::default();
            exporter
                .export(LogBatch::new(&[(&record, &scope)]))
                .await
                .map_err(ApiError::message)?;
            ctx.shutdown_node().await?;
            Ok(())
        });

        assert!(result.is_ok());
    }
}
