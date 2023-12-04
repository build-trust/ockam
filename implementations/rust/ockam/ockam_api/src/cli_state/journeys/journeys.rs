use crate::journeys::JourneyEvent;
use crate::logs::{CurrentSpan, OCKAM_TRACER_NAME};
use crate::{CliState, ProjectJourney};
use crate::{HostJourney, Result};
use chrono::{DateTime, Utc};
use ockam_node::OpenTelemetryContext;
use opentelemetry::trace::{Link, SpanBuilder, TraceContextExt, TraceId, Tracer};
use opentelemetry::{global, Context, Key};
use opentelemetry_sdk::trace::{IdGenerator, RandomIdGenerator};
use std::collections::HashMap;
use std::ops::Add;
use std::time::{Duration, SystemTime};

pub const USER_NAME: &Key = &Key::from_static_str("app.user_name");
pub const USER_EMAIL: &Key = &Key::from_static_str("app.user_email");
pub const APP_NAME: &Key = &Key::from_static_str("app.name");
pub const NODE_NAME: &Key = &Key::from_static_str("app.node_name");

pub const APPLICATION_EVENT_TRACE_ID: &Key = &Key::from_static_str("app.event.trace_id");
pub const APPLICATION_EVENT_SPAN_ID: &Key = &Key::from_static_str("app.event.span_id");
pub const APPLICATION_EVENT_TIMESTAMP: &Key = &Key::from_static_str("app.event.timestamp");
pub const APPLICATION_EVENT_PROJECT_ID: &Key = &Key::from_static_str("app.event.project_id");

/// The CliState can register events for major actions happening during the application execution:
///
///   - user commands: project creation, node creation, etc...
///   - connection events: portal connection established, Kafka consumer / producer connection established, etc...
///   - command errors
///
/// For each events we try set relevant attributes in order to get more context: node name, user email, host name etc...
/// Moreover we collect events as spans under 2 traces:
///
///  - a host trace: with all the events happening for a given host. That trace has a random trace id when created
///    and that trace persists even across resets
///
///  - a project trace: with all the events happening for a given project (so potentially across hosts).
///    That trace has a trace id based on the project id and it persists even across resets.
///
///
impl CliState {
    pub async fn add_journey_event(
        &self,
        event: JourneyEvent,
        attributes: HashMap<&Key, &str>,
    ) -> Result<()> {
        if !self.is_tracing_enabled() {
            return Ok(());
        }

        // get the journey context
        let tracer = global::tracer(OCKAM_TRACER_NAME);
        let event_span_context = Context::current().span().span_context().clone();
        let event_trace_id = event_span_context.trace_id();
        let event_span_id = event_span_context.span_id();
        let project_id = self.get_default_project().await.ok().map(|p| p.id);

        // for both the host and the project journey create a span with a fixed duration
        // and add attributes to the span
        let journeys = self.get_journeys().await?;
        for journey in journeys {
            let mut span_builder = SpanBuilder::from_name(event.to_string());
            span_builder.start_time = Some(journey.start_system_time());
            span_builder.end_time = Some(journey.start_system_time().add(Duration::from_millis(1)));
            span_builder.links = Some(vec![Link::new(event_span_context.clone(), vec![])]);
            let span = tracer.build_with_context(span_builder, &journey.extract_context());
            let cx = Context::current_with_span(span);
            let _guard = cx.attach();
            for (name, value) in attributes.iter() {
                CurrentSpan::set_attribute(name, value)
            }
            CurrentSpan::set_attribute(
                APPLICATION_EVENT_TRACE_ID,
                event_trace_id.to_string().as_ref(),
            );
            CurrentSpan::set_attribute(
                APPLICATION_EVENT_SPAN_ID,
                event_span_id.to_string().as_ref(),
            );
            CurrentSpan::set_attribute_time(APPLICATION_EVENT_TIMESTAMP);
            if let Some(project_id) = project_id.as_ref() {
                CurrentSpan::set_attribute(APPLICATION_EVENT_PROJECT_ID, project_id);
            }
        }
        Ok(())
    }

    /// Return a list of journeys for which we want to add spans
    async fn get_journeys(&self) -> Result<Vec<HostJourney>> {
        let repository = self.user_journey_repository();
        let mut result = vec![];
        if let Some(host_journey) = repository.get_host_journey().await? {
            result.push(host_journey)
        } else {
            let host_journey = self.create_host_journey();
            repository.store_host_journey(host_journey.clone()).await?;
            result.push(host_journey)
        }

        if let Ok(project) = self.get_default_project().await {
            if let Some(user_journey) = repository.get_project_journey(&project.id).await? {
                result.push(user_journey.to_host_journey())
            } else {
                let user_journey = self.create_project_journey(&project.id);
                repository
                    .store_project_journey(user_journey.clone())
                    .await?;
                result.push(user_journey.to_host_journey())
            }
        };

        Ok(result)
    }

    pub async fn reset_project_journey(&self, project_id: &str) -> Result<()> {
        let repository = self.user_journey_repository().await?;
        Ok(repository.delete_project_journey(project_id).await?)
    }

    /// Create the initial host journey, with a random trace id
    fn create_host_journey(&self) -> HostJourney {
        let (opentelemetry_context, now) = self.create_journey(
            "start host journey",
            RandomIdGenerator::default().new_trace_id(),
        );
        HostJourney::new(opentelemetry_context, now)
    }

    /// Create the initial project journey, with a trace id based on the project id
    fn create_project_journey(&self, project_id: &str) -> ProjectJourney {
        let project_id_trace_id = project_id.replace('-', "");
        let (opentelemetry_context, now) = self.create_journey(
            "start project journey",
            TraceId::from_hex(&project_id_trace_id).unwrap(),
        );
        ProjectJourney::new(project_id, opentelemetry_context, now)
    }

    /// Create the elements required for a journey:
    ///  - An OpenTelemetryContext, containing a trace id and a root span id.
    ///  - A start date. The start date is used when adding spans to the trace so that we add spans starting at the same
    ///    time. This will display aligned spans in the trace and we can recover the actual timestamp of the event with
    ///    an attribute
    fn create_journey(
        &self,
        msg: &str,
        trace_id: TraceId,
    ) -> (OpenTelemetryContext, DateTime<Utc>) {
        let tracer = global::tracer(OCKAM_TRACER_NAME);
        let (span, now) = Context::map_current(|cx| {
            let now = cx
                .get::<DateTime<Utc>>()
                .unwrap_or(&Utc::now())
                .add(Duration::from_millis(100));
            let mut span_builder = SpanBuilder::from_name(msg.to_string());
            span_builder.trace_id = Some(trace_id);
            span_builder.span_id = Some(RandomIdGenerator::default().new_span_id());
            span_builder.start_time = Some(SystemTime::from(now));
            span_builder.end_time = Some(SystemTime::from(now).add(Duration::from_millis(1)));
            (
                tracer.build_with_context(span_builder, &Context::default()),
                now,
            )
        });
        let cx = Context::current_with_span(span);
        let _guard = cx.clone().attach();
        let opentelemetry_context = OpenTelemetryContext::current();
        (opentelemetry_context, now)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logs::{LoggingConfiguration, LoggingTracing};
    use crate::random_name;
    use ockam_node::Executor;
    use opentelemetry::trace::FutureExt;
    use opentelemetry_sdk::testing::logs::InMemoryLogsExporter;
    use opentelemetry_sdk::testing::trace::InMemorySpanExporter;
    use tempfile::NamedTempFile;

    #[test]
    fn test_create_journey_event() {
        let spans_exporter = InMemorySpanExporter::default();
        let logs_exporter = InMemoryLogsExporter::default();

        let tracing_guard = LoggingTracing::setup_with_exporters(
            spans_exporter.clone(),
            logs_exporter.clone(),
            None,
            LoggingConfiguration::off().set_crates(&["ockam_api"]),
            "test",
        );
        let tracer = global::tracer("ockam-test");
        let result = tracer.in_span("user event", |cx| {
            let _guard = cx.with_value(Utc::now()).attach();

            Executor::execute_future(
                async move {
                    let db_file = NamedTempFile::new().unwrap();
                    let cli_state_directory = db_file.path().parent().unwrap().join(random_name());
                    let cli = CliState::create(cli_state_directory)
                        .await
                        .unwrap()
                        .set_tracing_enabled();

                    let mut map = HashMap::new();
                    map.insert(USER_EMAIL, "etorreborre@yahoo.com");
                    map.insert(USER_NAME, "eric");
                    cli.add_journey_event(JourneyEvent::Enrolled, map.clone())
                        .await
                        .unwrap();
                    cli.add_journey_event(JourneyEvent::PortalCreated, map)
                        .await
                        .unwrap();
                }
                .with_current_context(),
            )
        });
        assert!(result.is_ok());

        tracing_guard.force_flush();
        let mut spans = spans_exporter.get_finished_spans().unwrap();
        spans.sort_by_key(|s| s.start_time);
        assert_eq!(spans.len(), 4);

        let span_names = spans.iter().map(|s| s.name.as_ref()).collect::<Vec<&str>>();
        assert_eq!(
            span_names,
            vec![
                "user event",
                "start host journey",
                "enrolled",
                "portal created"
            ]
        );
        // remove the first event
        spans.remove(0);

        // all user events have the same start/end times and have a duration of 1ms
        let first_span = spans.first().unwrap().clone();
        for span in spans {
            assert_eq!(span.start_time, first_span.start_time);
            assert_eq!(span.end_time, first_span.end_time);
            assert_eq!(span.start_time.add(Duration::from_millis(1)), span.end_time);
        }
    }
}
