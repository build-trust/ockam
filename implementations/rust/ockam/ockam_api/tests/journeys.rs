use chrono::Utc;
use ockam_api::journeys::{
    JourneyEvent, APPLICATION_EVENT_TIMESTAMP, EVENT_DURATION, USER_EMAIL, USER_NAME,
};
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
                map.insert(USER_EMAIL, "etorreborre@yahoo.com".to_string());
                map.insert(USER_NAME, "eric".to_string());
                cli.add_journey_event(JourneyEvent::Enrolled, map.clone())
                    .await
                    .unwrap();
                cli.add_journey_event(JourneyEvent::PortalCreated, map)
                    .await
                    .unwrap();
                cli.add_journey_error("command", "sorry".to_string(), HashMap::default())
                    .await
                    .unwrap();
            }
            .with_current_context(),
        )
    });
    if let Err(e) = result {
        panic!("{e:?}");
    }

    tracing_guard.force_flush();
    let mut spans = spans_exporter.get_finished_spans().unwrap();
    spans.sort_by_key(|s| s.start_time);
    assert_eq!(spans.len(), 11);

    let mut span_names = spans.iter().map(|s| s.name.as_ref()).collect::<Vec<&str>>();
    let mut expected = vec![
        "user event",
        "get_default_project",
        "✅ enrolled",
        "get_default_project",
        "get_default_project",
        "✅ portal created",
        "get_default_project",
        "get_default_project",
        "❌ command error",
        "get_default_project",
        "start host journey",
    ];

    // spans are not necessarily retrieved in a deterministic order
    span_names.sort();
    expected.sort();
    assert_eq!(span_names, expected);

    // collect only the user events spans
    spans.retain(|s| {
        s.attributes
            .iter()
            .any(|kv| &kv.key == APPLICATION_EVENT_TIMESTAMP)
    });

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
