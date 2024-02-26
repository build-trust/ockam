use ockam_api::logs::{
    global_error_handler_enabled, Colored, LogFormat, LoggingConfiguration, LoggingEnabled,
    LoggingTracing, TracingConfiguration,
};
use ockam_api::random_name;

use opentelemetry::global;
use opentelemetry::trace::Tracer;

use opentelemetry_sdk::{self as sdk};
use sdk::testing::logs::*;
use sdk::testing::trace::*;

use std::fs;

use tempfile::NamedTempFile;

use tracing::{error, info};
use tracing_core::Level;

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
        &LoggingConfiguration::make_configuration()
            .unwrap()
            .set_log_directory(log_directory.into()),
        &TracingConfiguration::foreground(false).unwrap(),
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

/// HELPERS

fn make_configuration() -> ockam_core::Result<LoggingConfiguration> {
    Ok(LoggingConfiguration::new(
        LoggingEnabled::On,
        Level::TRACE,
        global_error_handler_enabled()?,
        100,
        60,
        LogFormat::Default,
        Colored::Off,
        None,
        None,
    ))
}
