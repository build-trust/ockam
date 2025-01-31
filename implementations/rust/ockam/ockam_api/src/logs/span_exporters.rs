use crate::cli_state::journeys::{APPLICATION_EVENT_NODE_NAME, APPLICATION_EVENT_OCKAM_DEVELOPER};
use crate::cli_state::CliStateError::InvalidData;
use crate::logs::http_forwarder::HTTP_FORWARDER;
use crate::logs::ockam_tonic_traces_client::OckamTonicTracesClient;
use crate::logs::secure_client_service::SecureClientService;
use crate::CliState;
use crate::Result;
use crate::{ApiError, TransportRouteResolver};
use futures::future::BoxFuture;
use ockam::identity::{get_default_timeout, SecureClient, TrustIdentifierPolicy};
use ockam_core::{async_trait, TryClone};
use ockam_node::Context;
use ockam_transport_tcp::TcpTransport;
use opentelemetry::{ExportError, KeyValue};
use opentelemetry_sdk::export::trace::{ExportResult, SpanData, SpanExporter};
use std::sync::Arc;
use std::time::Duration;
use tonic::codec::CompressionEncoding;

/// This exporter can be used to intercept the spans sent to an OpenTelemetry collector
#[derive(Debug)]
pub struct DecoratedSpanExporter<S: SpanExporter> {
    exporter: S,
}

#[async_trait]
impl<S: SpanExporter> SpanExporter for DecoratedSpanExporter<S> {
    fn export(&mut self, batch: Vec<SpanData>) -> BoxFuture<'static, ExportResult> {
        self.exporter.export(batch)
    }

    fn shutdown(&mut self) {
        debug!("shutting down the span exporter");
        self.exporter.shutdown()
    }

    fn force_flush(&mut self) -> BoxFuture<'static, ExportResult> {
        debug!("flushing the span exporter");
        self.exporter.force_flush()
    }
}

impl<S: SpanExporter> DecoratedSpanExporter<S> {
    pub fn new(exporter: S) -> DecoratedSpanExporter<S> {
        DecoratedSpanExporter { exporter }
    }
}

/// This exporter can be used to intercept the spans sent to an OpenTelemetry collector
/// and add custom attributes
#[derive(Debug)]
pub struct OckamSpanExporter<S: SpanExporter> {
    exporter: S,
    node_name: Option<String>,
    is_ockam_developer: bool,
    span_export_cutoff: Option<Duration>,
}

#[async_trait]
impl<S: SpanExporter> SpanExporter for OckamSpanExporter<S> {
    fn export(&mut self, batch: Vec<SpanData>) -> BoxFuture<'static, ExportResult> {
        let f = self.exporter.export(self.add_attributes(
            batch,
            self.node_name.clone(),
            self.is_ockam_developer,
        ));
        let span_export_cutoff = self.span_export_cutoff;

        Box::pin(async move {
            match span_export_cutoff {
                Some(cutoff) => {
                    let _ = tokio::time::timeout(cutoff, f).await;
                    Ok(())
                }
                None => f.await,
            }
        })
    }

    fn shutdown(&mut self) {
        debug!("shutting down the span exporter");
        self.exporter.shutdown()
    }

    fn force_flush(&mut self) -> BoxFuture<'static, ExportResult> {
        debug!("flushing the span exporter");
        self.exporter.force_flush()
    }
}

impl<S: SpanExporter> OckamSpanExporter<S> {
    pub fn new(
        exporter: S,
        node_name: Option<String>,
        is_ockam_developer: bool,
        span_export_cutoff: Option<Duration>,
    ) -> OckamSpanExporter<S> {
        OckamSpanExporter {
            exporter,
            node_name,
            is_ockam_developer,
            span_export_cutoff,
        }
    }

    fn add_attributes(
        &self,
        batch: Vec<SpanData>,
        node_name: Option<String>,
        is_ockam_developer: bool,
    ) -> Vec<SpanData> {
        batch
            .into_iter()
            .map(|s| self.add_attributes_to_span(s, node_name.clone(), is_ockam_developer))
            .collect()
    }

    fn add_attributes_to_span(
        &self,
        mut span: SpanData,
        node_name: Option<String>,
        is_ockam_developer: bool,
    ) -> SpanData {
        if let Some(node_name) = node_name {
            span.attributes.push(KeyValue::new(
                APPLICATION_EVENT_NODE_NAME.clone(),
                node_name,
            ));
        };
        span.attributes.push(KeyValue::new(
            APPLICATION_EVENT_OCKAM_DEVELOPER.clone(),
            is_ockam_developer,
        ));
        span
    }
}

#[derive(Debug)]
struct SecureChannelExporter {
    cli_state: CliState,
    ctx: Option<Context>,
    tcp_transport: Arc<TcpTransport>,
}

impl SecureChannelExporter {
    pub fn new(
        cli_state: CliState,
        ctx: &Context,
        tcp_transport: Arc<TcpTransport>,
    ) -> SecureChannelExporter {
        SecureChannelExporter {
            cli_state: cli_state.clone(),
            ctx: ctx.try_clone().ok(),
            tcp_transport: tcp_transport.clone(),
        }
    }

    async fn make_ockam_trace_client(
        &self,
        compression: Option<CompressionEncoding>,
    ) -> Result<OckamTonicTracesClient> {
        if let Some(ctx) = &self.ctx {
            let secure_client_service = SecureClientService::new(
                self.make_project_node_client().await?,
                &ctx,
                HTTP_FORWARDER,
            );
            Ok(OckamTonicTracesClient::new(
                secure_client_service,
                compression,
            ))
        } else {
            Err(ApiError::message(
                "cannot create a trace client without a context",
            ))
        }
    }

    async fn make_project_node_client(&self) -> Result<SecureClient> {
        let project = self.cli_state.projects().get_default_project().await?;
        let project_route = TransportRouteResolver::default()
            .allow_tcp()
            .resolve(project.project_multiaddr()?)?;
        let project_identifier =
            project
                .project_identifier()
                .ok_or(ApiError::CliState(InvalidData(
                    "The default project must have a name".into(),
                )))?;
        let default_node = self.cli_state.get_default_node().await?;
        let node_identifier = default_node.identifier();
        let secure_channels = self.cli_state.secure_channels(&default_node.name()).await?;

        Ok(SecureClient::new(
            secure_channels,
            None,
            self.tcp_transport.clone(),
            project_route,
            Arc::new(TrustIdentifierPolicy::new(project_identifier)),
            &node_identifier,
            get_default_timeout(),
            get_default_timeout(),
        ))
    }
}

impl Clone for SecureChannelExporter {
    fn clone(&self) -> Self {
        let ctx_clone = if let Some(ctx) = &self.ctx {
            ctx.try_clone().ok()
        } else {
            None
        };

        SecureChannelExporter {
            cli_state: self.cli_state.clone(),
            ctx: ctx_clone,
            tcp_transport: self.tcp_transport.clone(),
        }
    }
}

impl ExportError for ApiError {
    fn exporter_name(&self) -> &'static str {
        "ockam"
    }
}
