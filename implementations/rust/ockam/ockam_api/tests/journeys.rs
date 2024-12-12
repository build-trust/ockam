use chrono::Utc;
use ockam_api::cli_state::journeys::{
    JourneyEvent, APPLICATION_EVENT_TIMESTAMP, EVENT_DURATION, USER_EMAIL, USER_NAME,
};
use ockam_api::logs::{ExportingConfiguration, LoggingConfiguration, LoggingTracing};
use ockam_api::CliState;
use ockam_node::Executor;
use opentelemetry::global;
use opentelemetry::trace::{FutureExt, Tracer};
use opentelemetry_sdk::export::trace::SpanData;
use opentelemetry_sdk::testing::logs::InMemoryLogsExporter;
use opentelemetry_sdk::testing::trace::InMemorySpanExporter;
use std::collections::HashMap;
use std::ops::Add;

use ockam_api::cli_state::random_name;
use tempfile::NamedTempFile;

/// This test needs to be an integration test
/// It needs to run in isolation because
/// it sets up some global spans / logs exporters that might interact with other tests
#[test]
fn test_create_journey_event() {
    let cli = Executor::execute_future(async {
        let db_file = NamedTempFile::new().unwrap();
        let cli_state_directory = db_file.path().parent().unwrap().join(random_name());
        CliState::create(cli_state_directory)
            .await
            .unwrap()
            .set_tracing_enabled(true)
    })
    .unwrap();

    let spans_exporter = InMemorySpanExporter::default();
    let logs_exporter = InMemoryLogsExporter::default();

    let tracing_guard = LoggingTracing::setup_with_exporters(
        spans_exporter.clone(),
        logs_exporter.clone(),
        &LoggingConfiguration::off()
            .unwrap()
            .set_crates(&["ockam_api"]),
        &ExportingConfiguration::foreground(&cli).unwrap(),
        "test",
        None,
    );
    let tracer = global::tracer("ockam-test");
    let result = tracer.in_span("user event", |cx| {
        let _guard = cx.with_value(Utc::now()).attach();

        Executor::execute_future(
            async move {
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

    // keep only application events
    let spans: Vec<SpanData> = spans
        .into_iter()
        .filter(|s| {
            s.attributes
                .iter()
                .any(|kv| &kv.key == APPLICATION_EVENT_TIMESTAMP)
        })
        .collect();

    let mut span_names = spans.iter().map(|s| s.name.as_ref()).collect::<Vec<&str>>();
    let mut expected = vec!["✅ enrolled", "✅ portal created", "❌ command error"];

    // spans are not necessarily retrieved in a deterministic order
    span_names.sort();
    expected.sort();
    assert_eq!(span_names, expected);

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
