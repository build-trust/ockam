use chrono::{DateTime, Utc};
use ockam_core::OpenTelemetryContext;
use std::time::SystemTime;

/// A journey is a pseudo-trace where spans represents user events.
/// The tracing context is kept in the `opentelemetry_context` field.
///
/// A journey is time-limited to avoid spans accumulating for too long in a single trace.
/// After a while a new Journey is started and its `previous_opentelemetry_context` field must
/// point to the previous journey.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Journey {
    opentelemetry_context: OpenTelemetryContext,
    previous_opentelemetry_context: Option<OpenTelemetryContext>,
    start: DateTime<Utc>,
}

impl Journey {
    pub fn new(
        opentelemetry_context: OpenTelemetryContext,
        previous_opentelemetry_context: Option<OpenTelemetryContext>,
        start: DateTime<Utc>,
    ) -> Journey {
        Journey {
            opentelemetry_context,
            previous_opentelemetry_context,
            start,
        }
    }

    pub fn opentelemetry_context(&self) -> OpenTelemetryContext {
        self.opentelemetry_context.clone()
    }

    pub fn previous_opentelemetry_context(&self) -> Option<OpenTelemetryContext> {
        self.previous_opentelemetry_context.clone()
    }

    pub fn start(&self) -> DateTime<Utc> {
        self.start
    }

    pub fn start_system_time(&self) -> SystemTime {
        SystemTime::from(self.start)
    }

    pub fn extract_context(&self) -> opentelemetry::Context {
        self.opentelemetry_context.extract()
    }
}
