use crate::cli_state::journeys::attributes::make_host;
use crate::cli_state::journeys::{
    APPLICATION_EVENT_HOST, APPLICATION_EVENT_NODE_IDENTIFIER, APPLICATION_EVENT_NODE_NAME,
    APPLICATION_EVENT_OCKAM_DEVELOPER,
};
use crate::CliState;
use futures::future::BoxFuture;
use futures::FutureExt;
use ockam_core::async_trait;
use opentelemetry::KeyValue;
use opentelemetry_sdk::export::trace::{ExportResult, SpanData, SpanExporter};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

/// This exporter can be used to intercept the spans sent to an OpenTelemetry collector
#[derive(Debug)]
pub struct DecoratedSpanExporter<S: SpanExporter> {
    exporter: S,
}

#[async_trait]
impl<S: SpanExporter> SpanExporter for DecoratedSpanExporter<S> {
    fn export(&mut self, batch: Vec<SpanData>) -> BoxFuture<'static, ExportResult> {
        debug!("exporting {} spans", batch.len());
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
pub struct OckamSpanExporter<S: SpanExporter + 'static> {
    cli_state: Arc<CliState>,
    exporter: Arc<Mutex<S>>,
    is_ockam_developer: bool,
    span_export_cutoff: Option<Duration>,
}

#[async_trait]
impl<S: SpanExporter + 'static> SpanExporter for OckamSpanExporter<S> {
    fn export(&mut self, batch: Vec<SpanData>) -> BoxFuture<'static, ExportResult> {
        let cli_state = self.cli_state.clone();
        let is_ockam_developer = self.is_ockam_developer;
        let span_export_cutoff = self.span_export_cutoff;
        let exporter = self.exporter.clone();

        let f = async move {
            let mut exporter = exporter.lock().await;
            exporter
                .export(
                    Self::add_attributes(cli_state, Self::filter(batch), is_ockam_developer).await,
                )
                .await
        }
        .boxed();

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
        let mut exporter = self.exporter.blocking_lock(); // Use blocking_lock() to acquire a lock synchronously
        exporter.shutdown();
    }

    fn force_flush(&mut self) -> BoxFuture<'static, ExportResult> {
        debug!("flushing the span exporter");
        let exporter = self.exporter.clone();
        async move {
            let mut exporter = exporter.lock().await;
            exporter.force_flush().await
        }
        .boxed()
    }
}

impl<S: SpanExporter> OckamSpanExporter<S> {
    pub fn new(
        cli_state: Arc<CliState>,
        exporter: S,
        is_ockam_developer: bool,
        span_export_cutoff: Option<Duration>,
    ) -> OckamSpanExporter<S> {
        OckamSpanExporter {
            cli_state,
            exporter: Arc::new(Mutex::new(exporter)),
            is_ockam_developer,
            span_export_cutoff,
        }
    }

    async fn add_attributes(
        cli_state: Arc<CliState>,
        batch: Vec<SpanData>,
        is_ockam_developer: bool,
    ) -> Vec<SpanData> {
        let mut result = vec![];
        for span in batch.into_iter() {
            result.push(
                Self::add_attributes_to_span(cli_state.clone(), span, is_ockam_developer).await,
            )
        }
        result
    }

    async fn add_attributes_to_span(
        cli_state: Arc<CliState>,
        mut span: SpanData,
        is_ockam_developer: bool,
    ) -> SpanData {
        if let Ok(node_info) = cli_state.get_default_node().await {
            span.attributes.push(KeyValue::new(
                APPLICATION_EVENT_NODE_NAME.clone(),
                node_info.name(),
            ));
            span.attributes.push(KeyValue::new(
                APPLICATION_EVENT_NODE_IDENTIFIER.clone(),
                node_info.identifier().to_string(),
            ));
        };

        span.attributes.push(KeyValue::new(
            APPLICATION_EVENT_OCKAM_DEVELOPER.clone(),
            is_ockam_developer,
        ));
        span.attributes
            .push(KeyValue::new(APPLICATION_EVENT_HOST.clone(), make_host()));
        span
    }

    fn filter(batch: Vec<SpanData>) -> Vec<SpanData> {
        batch
            .into_iter()
            .filter_map(|s| Self::filter_span(s))
            .collect()
    }

    fn filter_span(mut span: SpanData) -> Option<SpanData> {
        // drop span events since they are log messages that we already send as logs records.
        span.events.events = vec![];
        Some(span)
    }
}
