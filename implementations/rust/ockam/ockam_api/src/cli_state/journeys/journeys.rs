use crate::cli_state::journeys::attributes::{
    default_attributes, make_host, make_host_trace_id, make_journey_span_id, make_project_trace_id,
};
use crate::cli_state::journeys::{Journey, JourneyEvent, ProjectJourney};
use crate::logs::CurrentSpan;
use crate::orchestrator::project::Project;
use crate::{CliState, Result};
use chrono::{DateTime, Utc};
use either::Either;
use ockam_core::{OpenTelemetryContext, OCKAM_TRACER_NAME};
use opentelemetry::trace::{Link, SpanBuilder, SpanId, TraceContextExt, TraceId, Tracer};
use opentelemetry::{global, Context, Key, KeyValue};
use std::collections::HashMap;
use std::ops::Add;
use std::time::{Duration, SystemTime};

pub const USER_NAME: &Key = &Key::from_static_str("app.user_name");
pub const USER_EMAIL: &Key = &Key::from_static_str("app.user_email");
pub const APP_NAME: &Key = &Key::from_static_str("app.name");
pub const NODE_NAME: &Key = &Key::from_static_str("app.node_name");
pub const IDENTIFIER: &Key = &Key::from_static_str("app.identifier");
pub const IDENTITY_NAME: &Key = &Key::from_static_str("app.identity_name");

pub const APPLICATION_EVENT_HOST: &Key = &Key::from_static_str("app.event.host");
pub const APPLICATION_EVENT_SPACE_ID: &Key = &Key::from_static_str("app.event.space.id");
pub const APPLICATION_EVENT_SPACE_NAME: &Key = &Key::from_static_str("app.event.space.name");
pub const APPLICATION_EVENT_PROJECT_ID: &Key = &Key::from_static_str("app.event.project.id");
pub const APPLICATION_EVENT_PROJECT_NAME: &Key = &Key::from_static_str("app.event.project.name");
pub const APPLICATION_EVENT_PROJECT_USER_ROLES: &Key =
    &Key::from_static_str("app.event.project.user_roles");
pub const APPLICATION_EVENT_PROJECT_ACCESS_ROUTE: &Key =
    &Key::from_static_str("app.event.project.access_route");
pub const APPLICATION_EVENT_PROJECT_IDENTIFIER: &Key =
    &Key::from_static_str("app.event.project.identifier");
pub const APPLICATION_EVENT_PROJECT_AUTHORITY_ACCESS_ROUTE: &Key =
    &Key::from_static_str("app.event.project.authority_access_route");
pub const APPLICATION_EVENT_PROJECT_AUTHORITY_IDENTIFIER: &Key =
    &Key::from_static_str("app.event.project.authority_identifier");

pub const APPLICATION_EVENT_NODE_NAME: &Key = &Key::from_static_str("app.event.node_name");
pub const APPLICATION_EVENT_OCKAM_DEVELOPER: &Key =
    &Key::from_static_str("app.event.ockam_developer");
pub const APPLICATION_EVENT_TRACE_ID: &Key = &Key::from_static_str("app.event.trace_id");
pub const APPLICATION_EVENT_SPAN_ID: &Key = &Key::from_static_str("app.event.span_id");
pub const APPLICATION_EVENT_TIMESTAMP: &Key = &Key::from_static_str("app.event.timestamp");
pub const APPLICATION_EVENT_ERROR_MESSAGE: &Key = &Key::from_static_str("app.event.error_message");
pub const APPLICATION_EVENT_COMMAND: &Key = &Key::from_static_str("app.event.command");
pub const APPLICATION_EVENT_COMMAND_CONFIGURATION_FILE: &Key =
    &Key::from_static_str("app.event.command.configuration_file");
pub const APPLICATION_EVENT_OCKAM_HOME: &Key = &Key::from_static_str("app.event.ockam_home");
pub const APPLICATION_EVENT_OCKAM_VERSION: &Key = &Key::from_static_str("app.event.ockam_version");
pub const APPLICATION_EVENT_OCKAM_GIT_HASH: &Key =
    &Key::from_static_str("app.event.ockam_git_hash");

/// Journey events have a fixed duration
pub const EVENT_DURATION: Duration = Duration::from_secs(100);

/// Maximum duration for a Journey: 5 days
const DEFAULT_JOURNEY_MAX_DURATION: Duration = Duration::from_secs(5 * 86400);

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
    /// This method adds a successful event to the project/host journeys
    #[instrument(skip_all)]
    pub async fn add_journey_event(
        &self,
        event: JourneyEvent,
        attributes: HashMap<&Key, String>,
    ) -> Result<()> {
        self.add_a_journey_event(event, attributes).await
    }

    /// This method adds an error event to the project/host journeys
    #[instrument(skip_all)]
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

    /// Add a journey event
    ///  - the event is represented as a span of fixed duration
    ///  - it contains a link to the current execution trace
    ///  - it is enriched with many attributes when available: project id, OCKAM_HOME, ockam version, etc...
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
        let project = self.projects().get_default_project().await.ok();

        // for both the host and the project journey create a span with a fixed duration
        // and add attributes to the span
        let start_time = SystemTime::from(Utc::now());
        let end_time = start_time.add(EVENT_DURATION);

        let journeys = self
            .get_journeys(project.clone().map(|p| p.project_id().to_string()))
            .await?;
        for journey in journeys {
            let span_builder = SpanBuilder::from_name(event.to_string())
                .with_start_time(start_time)
                .with_end_time(end_time)
                .with_links(vec![Link::new(event_span_context.clone(), vec![], 0)]);
            let span = tracer.build_with_context(span_builder, &journey.extract_context());
            let cx = Context::current_with_span(span);
            let _guard = cx.attach();
            self.set_current_span_attributes(&event, &attributes, &project)
        }
        Ok(())
    }

    /// Add both attributes to the current span
    ///  - caller attributes
    ///  - project attributes
    ///  - build attributes
    ///  - environment attributes
    fn set_current_span_attributes(
        &self,
        event: &JourneyEvent,
        attributes: &HashMap<&Key, String>,
        project: &Option<Project>,
    ) {
        let mut attributes = attributes.clone();
        attributes.extend(default_attributes());

        let event_span_context = Context::current().span().span_context().clone();
        let event_trace_id = event_span_context.trace_id();
        let event_span_id = event_span_context.span_id();

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
        if let Some(project) = project.as_ref() {
            CurrentSpan::set_attribute(APPLICATION_EVENT_SPACE_ID, project.space_id());
            CurrentSpan::set_attribute(APPLICATION_EVENT_SPACE_NAME, project.space_name());
            CurrentSpan::set_attribute(APPLICATION_EVENT_PROJECT_NAME, project.name());
            CurrentSpan::set_attribute(APPLICATION_EVENT_PROJECT_ID, project.project_id());
            CurrentSpan::set_attribute(
                APPLICATION_EVENT_PROJECT_USER_ROLES,
                &project
                    .model()
                    .user_roles
                    .iter()
                    .map(|u| u.to_string())
                    .collect::<Vec<_>>()
                    .join(","),
            );
            if let Ok(project_multiaddr) = project.project_multiaddr() {
                CurrentSpan::set_attribute(
                    APPLICATION_EVENT_PROJECT_ACCESS_ROUTE,
                    &project_multiaddr.to_string(),
                );
            }
            if let Some(project_identifier) = project.project_identifier() {
                CurrentSpan::set_attribute(
                    APPLICATION_EVENT_PROJECT_IDENTIFIER,
                    &project_identifier.to_string(),
                );
            }
            if let Ok(authority_multiaddr) = project.authority_multiaddr() {
                CurrentSpan::set_attribute(
                    APPLICATION_EVENT_PROJECT_AUTHORITY_ACCESS_ROUTE,
                    &authority_multiaddr.to_string(),
                );
            }
            if let Some(authority_identifier) = project.authority_identifier() {
                CurrentSpan::set_attribute(
                    APPLICATION_EVENT_PROJECT_AUTHORITY_IDENTIFIER,
                    &authority_identifier.to_string(),
                );
            }
        }
    }

    /// Return a list of journeys for which we want to add spans
    async fn get_journeys(&self, project_id: Option<String>) -> Result<Vec<Journey>> {
        let now = *Context::current()
            .get::<DateTime<Utc>>()
            .unwrap_or(&Utc::now());

        let mut result = vec![];

        let max_duration = DEFAULT_JOURNEY_MAX_DURATION;
        let journey = match self.get_host_journey(now, max_duration).await? {
            Some(Either::Right(journey)) => journey,
            Some(Either::Left(journey)) => {
                self.create_host_journey(Some(journey.opentelemetry_context()), now)
                    .await?
            }
            None => self.create_host_journey(None, now).await?,
        };
        result.push(journey);

        if let Some(project_id) = project_id {
            let journey = match self
                .get_project_journey(&project_id, now, max_duration)
                .await?
            {
                Some(Either::Right(journey)) => journey,
                Some(Either::Left(journey)) => {
                    self.create_project_journey(
                        &project_id,
                        Some(journey.opentelemetry_context()),
                        now,
                    )
                    .await?
                }
                None => self.create_project_journey(&project_id, None, now).await?,
            };
            result.push(journey.to_journey());
        };

        Ok(result)
    }

    /// When a project is deleted the project journeys need to be restarted
    pub async fn reset_project_journey(&self, project_id: &str) -> Result<()> {
        let repository = self.user_journey_repository();
        Ok(repository.delete_project_journeys(project_id).await?)
    }

    /// Return the latest host journey unless it has expired
    async fn get_host_journey(
        &self,
        now: DateTime<Utc>,
        max_duration: Duration,
    ) -> Result<Option<Either<Journey, Journey>>> {
        if let Some(journey) = self.user_journey_repository().get_host_journey(now).await? {
            if journey.start().add(max_duration) >= now {
                Ok(Some(Either::Right(journey)))
            } else {
                Ok(Some(Either::Left(journey)))
            }
        } else {
            Ok(None)
        }
    }

    /// Return the latest project journey unless it has expired
    async fn get_project_journey(
        &self,
        project_id: &str,
        now: DateTime<Utc>,
        max_duration: Duration,
    ) -> Result<Option<Either<ProjectJourney, ProjectJourney>>> {
        if let Some(journey) = self
            .user_journey_repository()
            .get_project_journey(project_id, now)
            .await?
        {
            if journey.start().add(max_duration) >= now {
                Ok(Some(Either::Right(journey)))
            } else {
                Ok(Some(Either::Left(journey)))
            }
        } else {
            Ok(None)
        }
    }

    /// Create the initial host journey, with a trace id based on the current time
    async fn create_host_journey(
        &self,
        previous_opentelemetry_context: Option<OpenTelemetryContext>,
        now: DateTime<Utc>,
    ) -> Result<Journey> {
        let trace_id = make_host_trace_id(now);
        let span_id = make_journey_span_id(trace_id);
        let host = make_host();
        let opentelemetry_context = self.create_open_telemetry_context(
            "start host journey",
            trace_id,
            span_id,
            &[(APPLICATION_EVENT_HOST, host)],
            previous_opentelemetry_context.clone(),
            now,
        );
        let journey = Journey::new(opentelemetry_context, previous_opentelemetry_context, now);
        self.user_journey_repository()
            .store_host_journey(journey.clone())
            .await?;
        Ok(journey)
    }

    /// Create the initial project journey, with a trace id based on the project id
    async fn create_project_journey(
        &self,
        project_id: &str,
        previous_opentelemetry_context: Option<OpenTelemetryContext>,
        now: DateTime<Utc>,
    ) -> Result<ProjectJourney> {
        let trace_id = make_project_trace_id(project_id, now);
        let span_id = make_journey_span_id(trace_id);

        let opentelemetry_context = self.create_open_telemetry_context(
            "start project journey",
            trace_id,
            span_id,
            &[(APPLICATION_EVENT_PROJECT_ID, project_id.to_string())],
            previous_opentelemetry_context.clone(),
            now,
        );
        let journey = ProjectJourney::new(
            project_id,
            opentelemetry_context,
            previous_opentelemetry_context,
            now,
        );
        self.user_journey_repository()
            .store_project_journey(journey.clone())
            .await?;
        Ok(journey)
    }

    /// Create an OpenTelemetryContext, containing a trace id and a root span id.
    fn create_open_telemetry_context(
        &self,
        msg: &str,
        trace_id: TraceId,
        span_id: SpanId,
        attributes: &[(&Key, String)],
        previous_opentelemetry_context: Option<OpenTelemetryContext>,
        now: DateTime<Utc>,
    ) -> OpenTelemetryContext {
        let tracer = global::tracer(OCKAM_TRACER_NAME);
        let now = now.add(Duration::from_millis(100));
        let mut span_builder = SpanBuilder::from_name(msg.to_string())
            .with_trace_id(trace_id)
            .with_span_id(span_id)
            .with_attributes(
                attributes
                    .iter()
                    .map(|(k, v)| KeyValue::new((*k).clone(), v.clone())),
            )
            .with_start_time(SystemTime::from(now))
            .with_end_time(SystemTime::from(now).add(Duration::from_millis(1)));
        if let Some(previous_opentelemetry_context) = previous_opentelemetry_context {
            span_builder = span_builder.with_links(vec![Link::new(
                previous_opentelemetry_context
                    .extract()
                    .span()
                    .span_context()
                    .clone(),
                vec![],
                0,
            )])
        };
        let span = tracer.build_with_context(span_builder, &Context::default());
        let cx = Context::current_with_span(span);
        OpenTelemetryContext::inject(&cx)
    }
}
