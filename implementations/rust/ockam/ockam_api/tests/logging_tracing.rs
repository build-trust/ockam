use ockam_api::logs::{LoggingConfiguration, LoggingTracing};
use ockam_api::random_name;
use ockam_node::{Executor, OpenTelemetryContext};
use opentelemetry::global;
use opentelemetry::trace::{FutureExt, Tracer};
use opentelemetry_sdk::{self as sdk};
use sdk::testing::logs::*;
use sdk::testing::trace::*;
use std::fs;
use tempfile::NamedTempFile;
use tracing::{error, info};

/// These tests need to be integration tests
/// They need to run in isolation because
/// they set up some global spans / logs exporters that might interact with other tests

#[test]
fn test_log_and_traces() {
    let temp_file = NamedTempFile::new().unwrap();
    let log_directory = &temp_file.path().parent().unwrap().join(random_name());

    let spans_exporter = InMemorySpanExporter::default();
    let logs_exporter = InMemoryLogsExporter::default();
    let guard = LoggingTracing::setup_with_exporters(
        spans_exporter.clone(),
        logs_exporter.clone(),
        None,
        LoggingConfiguration::default().set_log_directory(log_directory.into()),
        "test",
    );

    let tracer = global::tracer("ockam-test");
    tracer.in_span("Logging::test", |_| {
        info!("inside span");
        error!("something went wrong!");
    });

    // check that the spans are exported
    guard.force_flush();
    let spans = spans_exporter.get_finished_spans().unwrap();
    assert_eq!(spans.len(), 1);
    let parent_span = spans.first().unwrap();

    // check that log records are exported
    let logs = logs_exporter.get_emitted_logs().unwrap();
    assert_eq!(logs.len(), 2);
    for log in logs {
        assert_eq!(
            log.clone().record.trace_context.map(|tc| tc.trace_id),
            Some(parent_span.span_context.trace_id()),
            "{log:?}\n{parent_span:?}"
        )
    }

    // read the content of the log file to make sure that log messages are there
    let mut stdout_file_checked = false;
    for file in fs::read_dir(log_directory).unwrap() {
        let file_path = file.unwrap().path();
        if file_path.to_string_lossy().contains("stdout") {
            let contents = fs::read_to_string(file_path).unwrap();
            assert!(
                contents.contains("INFO logging_tracing: inside span"),
                "{:?}",
                contents
            );
            assert!(
                contents.contains("ERROR logging_tracing: something went wrong!"),
                "{:?}",
                contents
            );
            stdout_file_checked = true
        }
    }

    assert!(
        stdout_file_checked,
        "the stdout log file must have been found and checked"
    )
}

/// This test essentially checks that the tracing context that we propagate to other systems contains
/// a proper span id.
#[test]
fn test_context_propagation() {
    let spans_exporter = InMemorySpanExporter::default();
    let logs_exporter = InMemoryLogsExporter::default();
    let guard = LoggingTracing::setup_with_exporters(
        spans_exporter.clone(),
        logs_exporter.clone(),
        None,
        LoggingConfiguration::off(),
        "test",
    );

    let tracer = global::tracer("ockam-test");
    let propagated_context = Executor::execute_future(async move {
        tracer
            .in_span("root", |_| {
                async { function().await }.with_current_context()
            })
            .await
    })
    .unwrap();

    // get the exported spans
    guard.force_flush();
    let mut spans = spans_exporter.get_finished_spans().unwrap();
    spans.reverse();

    // there must be 3 spans
    assert_eq!(spans.len(), 3);
    let span1 = spans.get(0).unwrap();
    let span2 = spans.get(1).unwrap();
    let span3 = spans.get(2).unwrap();

    // the spans must have proper parent / child relationships
    assert_eq!(span1.name, "root");

    assert_eq!(span2.name, "function");
    assert_eq!(span2.parent_span_id, span1.span_context.span_id());

    assert_eq!(span3.name, "nested_function");
    assert_eq!(span3.parent_span_id, span2.span_context.span_id());

    // the propagated context must use the span id of the most nested span id
    let context = propagated_context.as_map();
    let traceparent = context.get("traceparent").unwrap();
    assert_eq!(
        traceparent.to_string(),
        format!(
            "00-{}-{}-01",
            span1.span_context.trace_id(),
            span3.span_context.span_id()
        )
    );
}

#[tracing::instrument]
async fn function() -> OpenTelemetryContext {
    nested_function().await
}

#[tracing::instrument]
async fn nested_function() -> OpenTelemetryContext {
    OpenTelemetryContext::current()
}
