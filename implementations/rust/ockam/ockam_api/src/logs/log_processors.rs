use opentelemetry::logs::{LogResult, Severity};
use opentelemetry_sdk::export::logs::LogData;
use opentelemetry_sdk::logs::LogProcessor;

/// This LogProcessor does not perform any shutdown
/// because there is a deadlock condition in the opentelemetry library on shutdown
#[derive(Debug)]
pub struct NonBlockingLogProcessor<L: LogProcessor> {
    log_processor: L,
}

impl<L: LogProcessor> NonBlockingLogProcessor<L> {
    pub fn new(log_processor: L) -> NonBlockingLogProcessor<L> {
        NonBlockingLogProcessor { log_processor }
    }
}

impl<L: LogProcessor> LogProcessor for NonBlockingLogProcessor<L> {
    fn emit(&self, data: LogData) {
        self.log_processor.emit(data)
    }

    fn force_flush(&self) -> LogResult<()> {
        self.log_processor.force_flush()
    }

    /// We only flush instead of shutting down
    fn shutdown(&mut self) -> LogResult<()> {
        self.force_flush()
    }

    fn event_enabled(&self, level: Severity, target: &str, name: &str) -> bool {
        self.log_processor.event_enabled(level, target, name)
    }
}
