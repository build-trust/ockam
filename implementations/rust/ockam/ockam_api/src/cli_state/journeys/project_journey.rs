use crate::cli_state::journeys::Journey;
use chrono::{DateTime, Utc};
use ockam_core::OpenTelemetryContext;
use std::time::SystemTime;

/// A Project journey is a journey (i.e. a pseudo trace storing user events)
/// scoped to a specific project
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectJourney {
    project_id: String,
    journey: Journey,
}

impl ProjectJourney {
    pub fn new(
        project_id: &str,
        opentelemetry_context: OpenTelemetryContext,
        previous_opentelemetry_context: Option<OpenTelemetryContext>,
        start: DateTime<Utc>,
    ) -> ProjectJourney {
        ProjectJourney {
            project_id: project_id.to_string(),
            journey: Journey::new(opentelemetry_context, previous_opentelemetry_context, start),
        }
    }

    pub fn to_journey(&self) -> Journey {
        Journey::new(
            self.opentelemetry_context(),
            self.previous_opentelemetry_context(),
            self.journey.start(),
        )
    }

    pub fn opentelemetry_context(&self) -> OpenTelemetryContext {
        self.journey.opentelemetry_context()
    }

    pub fn previous_opentelemetry_context(&self) -> Option<OpenTelemetryContext> {
        self.journey.previous_opentelemetry_context()
    }

    pub fn start(&self) -> DateTime<Utc> {
        self.journey.start()
    }

    pub fn start_system_time(&self) -> SystemTime {
        SystemTime::from(self.start())
    }

    pub fn extract_context(&self) -> opentelemetry::Context {
        self.opentelemetry_context().extract()
    }

    pub fn project_id(&self) -> String {
        self.project_id.clone()
    }
}
