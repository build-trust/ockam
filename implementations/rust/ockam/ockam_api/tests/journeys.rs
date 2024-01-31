use chrono::Utc;
use ockam_api::journeys::{JourneyEvent, EVENT_DURATION, USER_EMAIL, USER_NAME};
use ockam_api::logs::{LoggingConfiguration, LoggingTracing};
use ockam_api::{random_name, CliState};
use ockam_node::Executor;
use opentelemetry::global;
use opentelemetry::trace::{FutureExt, Tracer};
use opentelemetry_sdk::testing::logs::InMemoryLogsExporter;
use opentelemetry_sdk::testing::trace::InMemorySpanExporter;
use std::collections::HashMap;
use std::ops::Add;

use tempfile::NamedTempFile;

/// This test needs to be an integration test
/// It needs to run in isolation because
/// it sets up some global spans / logs exporters that might interact with other tests
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
                cli.add_journey_error("command", "sorry".to_string())
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
    assert_eq!(spans.len(), 5);

    let span_names = spans.iter().map(|s| s.name.as_ref()).collect::<Vec<&str>>();
    assert_eq!(
        span_names,
        vec![
            "user event",
            "enrolled",
            "portal created",
            "command error",
            "start host journey",
        ]
    );
    // remove the first and last spans, which are starting traces
    spans.remove(0);
    spans.remove(3);

    // all user events have a fixed duration
    for span in spans {
        assert_eq!(
            span.start_time.add(EVENT_DURATION),
            span.end_time,
            "incorrect times for {}",
            span.name
        );
    }
}
