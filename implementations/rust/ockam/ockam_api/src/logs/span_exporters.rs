use crate::cli_state::journeys::{APPLICATION_EVENT_NODE_NAME, APPLICATION_EVENT_OCKAM_DEVELOPER};
use futures::future::BoxFuture;
use ockam_core::async_trait;
use opentelemetry::KeyValue;
use opentelemetry_sdk::export::trace::{ExportResult, SpanData, SpanExporter};
use std::time::Duration;

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
