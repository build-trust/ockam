use crate::journeys::JourneyEvent;
use crate::logs::{CurrentSpan, OCKAM_TRACER_NAME};
use crate::{CliState, ProjectJourney};
use crate::{HostJourney, Result};
use chrono::{DateTime, Utc};
use itertools::Itertools;
use ockam_node::OpenTelemetryContext;
use opentelemetry::trace::{Link, SpanBuilder, SpanId, TraceContextExt, TraceId, Tracer};
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
pub const APPLICATION_EVENT_ERROR_MESSAGE: &Key = &Key::from_static_str("app.event.error_message");
pub const APPLICATION_EVENT_COMMAND: &Key = &Key::from_static_str("app.event.command");
pub const APPLICATION_EVENT_OCKAM_HOME: &Key = &Key::from_static_str("app.event.ockam_home");
pub const APPLICATION_EVENT_OCKAM_VERSION: &Key = &Key::from_static_str("app.event.ockam_version");
pub const APPLICATION_EVENT_OCKAM_GIT_HASH: &Key =
    &Key::from_static_str("app.event.ockam_git_hash");

/// Journey events have a fixed duration
pub const EVENT_DURATION: Duration = Duration::from_secs(100);

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
    pub async fn add_journey_error(
        &self,
        command_name: &str,
        message: String,
        attributes: HashMap<&Key, String>,
    ) -> Result<()> {
        self.add_a_journey_event(
            JourneyEvent::error(command_name.to_string(), message),
            attributes,
        )
        .await
    }

    pub async fn add_journey_event(
        &self,
        event: JourneyEvent,
        attributes: HashMap<&Key, String>,
    ) -> Result<()> {
        self.add_a_journey_event(event, attributes).await
    }

    async fn add_a_journey_event(
        &self,
        event: JourneyEvent,
        attributes: HashMap<&Key, String>,
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
        let start_time = SystemTime::from(Utc::now());
        let end_time = start_time.add(EVENT_DURATION);

        let journeys = self.get_journeys().await?;
        for journey in journeys {
            let span_builder = SpanBuilder::from_name(event.to_string())
                .with_start_time(start_time)
                .with_end_time(end_time)
                .with_links(vec![Link::new(event_span_context.clone(), vec![])]);
            let span = tracer.build_with_context(span_builder, &journey.extract_context());
            let cx = Context::current_with_span(span);
            let _guard = cx.attach();

            for (name, value) in attributes.iter() {
                CurrentSpan::set_attribute(name, value)
            }
            if let JourneyEvent::Error { message, .. } = &event {
                CurrentSpan::set_attribute(&Key::from_static_str("error"), "true");
                CurrentSpan::set_attribute(APPLICATION_EVENT_ERROR_MESSAGE, message);
            };

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
        let repository = self.user_journey_repository();
        Ok(repository.delete_project_journey(project_id).await?)
    }

    /// Create the initial host journey, with a random trace id
    fn create_host_journey(&self) -> HostJourney {
        let random_id_generator = RandomIdGenerator::default();
        let (opentelemetry_context, now) = self.create_journey(
            "start host journey",
            random_id_generator.new_trace_id(),
            random_id_generator.new_span_id(),
        );
        HostJourney::new(opentelemetry_context, now)
    }

    /// Create the initial project journey, with a trace id based on the project id
    fn create_project_journey(&self, project_id: &str) -> ProjectJourney {
        // take the first part of the project id, until '-' as the span id
        let split = project_id.split('-').collect::<Vec<_>>();
        let project_id_span_id = split.iter().take(2).join("");
        let span_id = SpanId::from_hex(&project_id_span_id).unwrap();

        // take the whole project without '-' as the trace id
        let project_id_trace_id = project_id.replace('-', "");
        let trace_id = TraceId::from_hex(&project_id_trace_id).unwrap();
        let (opentelemetry_context, now) =
            self.create_journey("start project journey", trace_id, span_id);
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
        span_id: SpanId,
    ) -> (OpenTelemetryContext, DateTime<Utc>) {
        let tracer = global::tracer(OCKAM_TRACER_NAME);
        let (span, now) = Context::map_current(|cx| {
            let now = cx
                .get::<DateTime<Utc>>()
                .unwrap_or(&Utc::now())
                .add(Duration::from_millis(100));
            let mut span_builder = SpanBuilder::from_name(msg.to_string());
            span_builder.trace_id = Some(trace_id);
            span_builder.span_id = Some(span_id);
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
