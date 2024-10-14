use ockam_core::async_trait;
use opentelemetry::logs::{LogResult, Severity};
use opentelemetry_sdk::export::logs::{LogBatch, LogExporter};
use std::time::Duration;

/// This exporter can be used to intercept the log records sent to an OpenTelemetry collector
#[derive(Debug)]
pub struct DecoratedLogExporter<L: LogExporter> {
    exporter: L,
}

#[async_trait]
impl<L: LogExporter> LogExporter for DecoratedLogExporter<L> {
    async fn export(&mut self, batch: LogBatch<'_>) -> LogResult<()> {
        self.exporter.export(batch).await
    }

    fn shutdown(&mut self) {
        self.exporter.shutdown()
    }

    fn event_enabled(&self, level: Severity, target: &str, name: &str) -> bool {
        self.exporter.event_enabled(level, target, name)
    }
}

impl<L: LogExporter> DecoratedLogExporter<L> {
    pub fn new(exporter: L) -> DecoratedLogExporter<L> {
        DecoratedLogExporter { exporter }
    }
}

/// This exporter is used to avoid waiting on a full request/response roundtrip when sending
/// log records sent to an OpenTelemetry collector
#[derive(Debug)]
pub struct OckamLogExporter<L: LogExporter> {
    exporter: L,
    log_export_cutoff: Option<Duration>,
}

#[async_trait]
impl<L: LogExporter> LogExporter for OckamLogExporter<L> {
    async fn export(&mut self, batch: LogBatch<'_>) -> LogResult<()> {
        match self.log_export_cutoff {
            Some(cutoff) => {
                let f = self.exporter.export(batch);
                let _ = tokio::time::timeout(cutoff, f).await;
                Ok(())
            }
            None => self.exporter.export(batch).await,
        }
    }

    fn shutdown(&mut self) {
        self.exporter.shutdown()
    }

    fn event_enabled(&self, level: Severity, target: &str, name: &str) -> bool {
        self.exporter.event_enabled(level, target, name)
    }
}

impl<L: LogExporter> OckamLogExporter<L> {
    pub fn new(exporter: L, log_export_cutoff: Option<Duration>) -> OckamLogExporter<L> {
        OckamLogExporter {
            exporter,
            log_export_cutoff,
        }
    }
}
