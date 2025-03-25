use ockam_api::logs::{
    get_https_endpoint, Colored, CratesFilter, ExportingConfiguration, ExportingEnabled, LogFormat,
    LoggingConfiguration, LoggingEnabled, LoggingTracing,
};

use opentelemetry::global;
use opentelemetry::trace::Tracer;
use opentelemetry_sdk::testing::logs::InMemoryLogExporter;
use opentelemetry_sdk::testing::trace::InMemorySpanExporter;
use std::fs;
use std::sync::Arc;
use tempfile::NamedTempFile;

use ockam_api::cli_state::random_name;
use ockam_api::CliState;
use tracing::{error, info};
use tracing_core::Level;

/// These tests need to be integration tests
/// They need to run in isolation because
/// they set up some global spans / logs exporters that might interact with other tests
#[tokio::test]
async fn test_log_and_traces() {
    let temp_file = NamedTempFile::new().unwrap();
    let log_directory = &temp_file.path().parent().unwrap().join(random_name());
    let span_exporter = InMemorySpanExporter::default();
    let log_exporter = InMemoryLogExporter::default();
    let endpoint = get_https_endpoint().unwrap();
    let guard = LoggingTracing::setup_with_exporters(
        Arc::new(CliState::test().await.unwrap()),
        span_exporter.clone(),
        log_exporter.clone(),
        &make_configuration()
            .unwrap()
            .set_log_directory(log_directory.into()),
        &ExportingConfiguration::make_foreground_exporting_configuration(
            endpoint,
            ExportingEnabled::Off,
        )
        .unwrap(),
        "test",
        Some("node-name".into()),
    );

    let tracer = global::tracer("ockam-test");
    tracer.in_span("Logging::test", |_| {
        info!("inside span");
        error!("something went wrong!");
    });

    // check that the spans are exported
    guard.force_flush().await;

    let spans = span_exporter.get_finished_spans().unwrap();
    assert_eq!(spans.len(), 1);
    let parent_span = spans.first().unwrap();

    // check that log records are exported
    let logs = log_exporter.get_emitted_logs().unwrap();
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
                contents.contains("INFO inside span logging_tracing"),
                "{:?}",
                contents
            );
            assert!(
                contents.contains("ERROR something went wrong! logging_tracing"),
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

// HELPERS

fn make_configuration() -> ockam_core::Result<LoggingConfiguration> {
    Ok(LoggingConfiguration::new(
        LoggingEnabled::On,
        Level::TRACE,
        100,
        60,
        LogFormat::Default,
        Colored::Off,
        None,
        CratesFilter::All,
    ))
}
