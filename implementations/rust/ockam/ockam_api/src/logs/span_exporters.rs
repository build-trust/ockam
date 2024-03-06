use crate::journeys::APPLICATION_EVENT_NODE_NAME;
use futures::future::BoxFuture;
use ockam_core::async_trait;
use opentelemetry::KeyValue;
use opentelemetry_sdk::export::trace::{ExportResult, SpanData, SpanExporter};

/// This exporter can be used to intercept the spans sent to an OpenTelemetry collector
#[derive(Debug)]
pub struct DecoratedSpanExporter<S: SpanExporter> {
    pub exporter: S,
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

/// This exporter can be used to intercept the spans sent to an OpenTelemetry collector
#[derive(Debug)]
pub struct NodeSpanExporter<S: SpanExporter> {
    pub exporter: S,
    pub node_name: Option<String>,
}

#[async_trait]
impl<S: SpanExporter> SpanExporter for NodeSpanExporter<S> {
    fn export(&mut self, batch: Vec<SpanData>) -> BoxFuture<'static, ExportResult> {
        match &self.node_name {
            Some(node_name) => self
                .exporter
                .export(self.add_node_attribute(batch, node_name.clone())),
            None => self.exporter.export(batch),
        }
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

impl<S: SpanExporter> NodeSpanExporter<S> {
    pub fn new(exporter: S, node_name: Option<String>) -> NodeSpanExporter<S> {
        NodeSpanExporter {
            exporter,
            node_name,
        }
    }

    fn add_node_attribute(&self, batch: Vec<SpanData>, node_name: String) -> Vec<SpanData> {
        batch
            .into_iter()
            .map(|s| self.add_node_attribute_to_span(s, node_name.clone()))
            .collect()
    }

    fn add_node_attribute_to_span(&self, mut span: SpanData, node_name: String) -> SpanData {
        span.attributes.push(KeyValue::new(
            APPLICATION_EVENT_NODE_NAME.clone(),
            node_name,
        ));
        span
    }
}
