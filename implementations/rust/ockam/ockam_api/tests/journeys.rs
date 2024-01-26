use chrono::Utc;
use ockam_api::journeys::{JourneyEvent, USER_EMAIL, USER_NAME};
use ockam_api::logs::{LoggingConfiguration, LoggingTracing};
use ockam_api::{random_name, CliState};
use ockam_node::Executor;
use opentelemetry::global;
use opentelemetry::trace::{FutureExt, Tracer};
use opentelemetry_sdk::testing::logs::InMemoryLogsExporter;
use opentelemetry_sdk::testing::trace::InMemorySpanExporter;
use std::collections::HashMap;
use std::ops::Add;
use std::time::Duration;
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
        LoggingConfiguration::off(),
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
