use chrono::Utc;
use ockam_api::cli_state::journeys::{
    JourneyEvent, APPLICATION_EVENT_TIMESTAMP, EVENT_DURATION, USER_EMAIL, USER_NAME,
};
use ockam_api::logs::{ExportingConfiguration, LoggingConfiguration, LoggingTracing};
use ockam_api::CliState;
use opentelemetry::trace::{FutureExt, TraceContextExt, Tracer};
use opentelemetry::{global, Context};
use opentelemetry_sdk::export::trace::SpanData;
use opentelemetry_sdk::testing::logs::InMemoryLogsExporter;
use opentelemetry_sdk::testing::trace::InMemorySpanExporter;
use std::collections::HashMap;
use std::ops::Add;

use ockam_api::cli_state::{random_name, CliStateMode};
use tempfile::NamedTempFile;

/// This test needs to be an integration test
/// It needs to run in isolation because
/// it sets up some global spans / logs exporters that might interact with other tests
#[tokio::test]
async fn test_create_journey_event() {
    let db_file = NamedTempFile::new().unwrap();
    let cli_state_directory = db_file.path().parent().unwrap().join(random_name());
    let cli = CliState::create(CliStateMode::Persistent(cli_state_directory))
        .await
        .unwrap()
        .set_tracing_enabled(true);

    let spans_exporter = InMemorySpanExporter::default();
    let logs_exporter = InMemoryLogsExporter::default();

    let tracing_guard = LoggingTracing::setup_with_exporters(
        spans_exporter.clone(),
        logs_exporter.clone(),
        &LoggingConfiguration::off()
            .unwrap()
            .set_crates(&["ockam_api"]),
        &ExportingConfiguration::foreground(&cli).await.unwrap(),
        "test",
        None,
    );
    let tracer = global::tracer("ockam-test");
    let span = tracer.start("user event");
    let cx = Context::current_with_span(span);

    let _guard = cx.with_value(Utc::now()).attach();

    let mut map = HashMap::new();
    map.insert(USER_EMAIL, "etorreborre@yahoo.com".to_string());
    map.insert(USER_NAME, "eric".to_string());
    cli.add_journey_event(JourneyEvent::Enrolled, map.clone())
        .with_context(cx.clone())
        .await
        .unwrap();
    cli.add_journey_event(JourneyEvent::PortalCreated, map)
        .with_context(cx.clone())
        .await
        .unwrap();
    cli.add_journey_error("command", "sorry".to_string(), HashMap::default())
        .with_context(cx.clone())
        .await
        .unwrap();

    tracing_guard.force_flush().await;
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
