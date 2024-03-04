use ockam_core::async_trait;
use opentelemetry::logs::{LogResult, Severity};
use opentelemetry_sdk::export::logs::{LogData, LogExporter};

/// This exporter can be used to intercept the log records sent to an OpenTelemetry collector
#[derive(Debug)]
pub struct DecoratedLogExporter<L: LogExporter> {
    pub exporter: L,
}

#[async_trait]
impl<L: LogExporter> LogExporter for DecoratedLogExporter<L> {
    async fn export(&mut self, batch: Vec<LogData>) -> LogResult<()> {
        self.exporter.export(batch).await
    }

    fn shutdown(&mut self) {
        self.exporter.shutdown()
    }

    fn event_enabled(&self, level: Severity, target: &str, name: &str) -> bool {
        self.exporter.event_enabled(level, target, name)
    }
}
